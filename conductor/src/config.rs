use crate::types::{ConnectionName, InterfaceName, MachineName, SystemName};
use conductor_config::{
    ConnectorPropertiesError, GpioConnectorProperties, MachineBackend, NetworkConnectorProperties,
    UartConnectorProperties,
};
use derive_more::From;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("A connection must have a name")]
    EmptyConnectionName,
    #[error("Found connections with the same name '{_0}'")]
    DupConnection(ConnectionName),
    #[error("A connector must have a name that refers to a connection")]
    EmptyConnectorName,
    #[error("A connector must have an interface")]
    EmptyConnectorInterface,
    #[error("Machine '{_0}' has a invalid duplicate connector '{_1}'")]
    DupConnector(MachineName, ConnectionName),
    #[error("A machine connector references a connection '{_0}' that isn't defined")]
    MissingConnectorConnection(ConnectionName),
    #[error("A machine must have a name")]
    EmptyMachineName,
    #[error("The host binary '{_0:?}' for machine '{_1}' does not exist")]
    NonExistentMachineBin(PathBuf, MachineName),
    #[error("The host asset '{_0:?}' for machine '{_1}' does not exist")]
    NonExistentMachineAsset(PathBuf, MachineName),
    #[error("Machine '{_0}' does not have a backend specified")]
    NoMachineBackend(MachineName),
    #[error("Machine '{_0}' does not have a bin path specified")]
    NoMachineBin(MachineName),
    #[error("Found duplicate machines with name '{_0}'")]
    DupMachine(MachineName),
    #[error(transparent)]
    ConnectorProperties(#[from] ConnectorPropertiesError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigReadError {
    #[error(transparent)]
    Syntax(#[from] conductor_config::ConfigReadError),
    #[error(transparent)]
    Semantics(#[from] ConfigError),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Config {
    pub global: Global,
    // TODO
    //pub simulators: Vec<Simulator>,
    pub machines: BTreeSet<Machine>,
    pub connections: BTreeSet<Connection>,
    // TODO
    //pub storages: Vec<Storage>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Global {
    pub name: SystemName,
    pub environment_variables: BTreeMap<String, String>,
}

// TODO
// pub struct Simulator {
// pub enum SimulatorBackend {
// pub struct GazeboSimulatorBackend {

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Machine {
    pub name: MachineName,
    pub bin: PathBuf,
    pub environment_variables: BTreeMap<String, String>,
    pub assets: BTreeMap<PathBuf, PathBuf>,
    pub backend: MachineBackend,
    pub connectors: BTreeSet<MachineConnector>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MachineConnector {
    pub name: ConnectionName,
    pub interface: InterfaceName,
    pub properties: ConnectorProperties,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub enum ConnectorProperties {
    Uart(UartConnectorProperties),
    Gpio(GpioConnectorProperties),
    Network(NetworkConnectorProperties),
}

// TODO(jon@auxon.io) add util helpers
// symmetrical or asymmetrical
// guest-to-guest
// guest-to-host
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ConnectionKind {
    Uart,
    Gpio,
    Network,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Connection {
    Uart(UartConnection),
    Gpio(GpioConnection),
    Network(NetworkConnection),
}

impl Connection {
    pub fn name(&self) -> &ConnectionName {
        use Connection::*;
        match self {
            Uart(c) => &c.name,
            Gpio(c) => &c.name,
            Network(c) => &c.name,
        }
    }

    pub fn kind(&self) -> ConnectionKind {
        use Connection::*;
        match self {
            Uart(_) => ConnectionKind::Uart,
            Gpio(_) => ConnectionKind::Gpio,
            Network(_) => ConnectionKind::Network,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UartConnection {
    pub name: ConnectionName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct GpioConnection {
    pub name: ConnectionName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct NetworkConnection {
    pub name: ConnectionName,
}

impl From<conductor_config::Global> for Global {
    fn from(value: conductor_config::Global) -> Self {
        Self {
            name: value
                .name
                .as_ref()
                .and_then(SystemName::new)
                .unwrap_or_default(),
            environment_variables: value.environment_variables,
        }
    }
}

impl TryFrom<(conductor_config::Machine, &BTreeSet<Connection>)> for Machine {
    type Error = ConfigError;

    fn try_from(
        values: (conductor_config::Machine, &BTreeSet<Connection>),
    ) -> Result<Self, Self::Error> {
        let (value, connections) = values;
        let name = value
            .name
            .as_ref()
            .and_then(MachineName::new)
            .ok_or(ConfigError::EmptyMachineName)?;
        let bin = value
            .bin
            .ok_or_else(|| ConfigError::NoMachineBin(name.clone()))?;
        if !bin.exists() {
            return Err(ConfigError::NonExistentMachineBin(bin, name));
        }
        for host_asset in value.assets.keys() {
            if !host_asset.exists() {
                return Err(ConfigError::NonExistentMachineAsset(
                    host_asset.clone(),
                    name,
                ));
            }
        }
        let backend = value
            .backend
            .ok_or_else(|| ConfigError::NoMachineBackend(name.clone()))?;
        let mut connectors = BTreeSet::new();
        for c in value.connectors.into_iter() {
            let c = MachineConnector::try_from((c, connections))?;
            if let Some(prev_con) = connectors.replace(c) {
                return Err(ConfigError::DupConnector(name, prev_con.name));
            }
        }
        Ok(Self {
            name,
            bin,
            environment_variables: value.environment_variables,
            assets: value.assets,
            backend,
            connectors,
        })
    }
}

impl TryFrom<(conductor_config::MachineConnector, &BTreeSet<Connection>)> for MachineConnector {
    type Error = ConfigError;

    fn try_from(
        values: (conductor_config::MachineConnector, &BTreeSet<Connection>),
    ) -> Result<Self, Self::Error> {
        let (value, connections) = values;
        let name = ConnectionName::new(&value.name).ok_or(ConfigError::EmptyConnectorName)?;
        let interface =
            InterfaceName::new(&value.interface).ok_or(ConfigError::EmptyConnectorInterface)?;
        let connection = connections
            .iter()
            .find(|c| c.name() == &name)
            .ok_or_else(|| ConfigError::MissingConnectorConnection(name.clone()))?;
        let properties = match connection.kind() {
            ConnectionKind::Uart => UartConnectorProperties::try_from(&value)?.into(),
            ConnectionKind::Gpio => GpioConnectorProperties::try_from(&value)?.into(),
            ConnectionKind::Network => NetworkConnectorProperties::try_from(&value)?.into(),
        };
        Ok(Self {
            name,
            interface,
            properties,
        })
    }
}

impl TryFrom<conductor_config::Connection> for Connection {
    type Error = ConfigError;

    fn try_from(value: conductor_config::Connection) -> Result<Self, Self::Error> {
        Ok(match value {
            conductor_config::Connection::Uart(c) => Connection::Uart(UartConnection::try_from(c)?),
            conductor_config::Connection::Gpio(c) => Connection::Gpio(GpioConnection::try_from(c)?),
            conductor_config::Connection::Network(c) => {
                Connection::Network(NetworkConnection::try_from(c)?)
            }
        })
    }
}

impl TryFrom<conductor_config::UartConnection> for UartConnection {
    type Error = ConfigError;

    fn try_from(value: conductor_config::UartConnection) -> Result<Self, Self::Error> {
        Ok(Self {
            name: ConnectionName::new(value.name).ok_or(ConfigError::EmptyConnectionName)?,
        })
    }
}

impl TryFrom<conductor_config::GpioConnection> for GpioConnection {
    type Error = ConfigError;

    fn try_from(value: conductor_config::GpioConnection) -> Result<Self, Self::Error> {
        Ok(Self {
            name: ConnectionName::new(value.name).ok_or(ConfigError::EmptyConnectionName)?,
        })
    }
}

impl TryFrom<conductor_config::NetworkConnection> for NetworkConnection {
    type Error = ConfigError;

    fn try_from(value: conductor_config::NetworkConnection) -> Result<Self, Self::Error> {
        Ok(Self {
            name: ConnectionName::new(value.name).ok_or(ConfigError::EmptyConnectionName)?,
        })
    }
}

impl Config {
    pub fn read<P: AsRef<Path>>(config_path: P) -> Result<Self, ConfigReadError> {
        let cfg = conductor_config::Config::read(config_path)?;
        // TODO(jon@auxon.io)
        // basic top-level validation
        // names exist
        // backends are provided
        // resolve and check connectors to their connections
        // ...

        let mut connections = BTreeSet::new();
        for c in cfg.connections.into_iter() {
            let c = Connection::try_from(c)?;
            if let Some(prev_con) = connections.replace(c) {
                return Err(ConfigError::DupConnection(prev_con.name().clone()).into());
            }
        }

        let mut machines: BTreeSet<Machine> = BTreeSet::new();
        for m in cfg.machines.into_iter() {
            let m = Machine::try_from((m, &connections))?;

            let contains_name_already = machines.iter().any(|known_m| known_m.name == m.name);
            if contains_name_already {
                return Err(ConfigError::DupMachine(m.name).into());
            }

            if let Some(prev_m) = machines.replace(m) {
                return Err(ConfigError::DupMachine(prev_m.name).into());
            }
        }

        Ok(Self {
            global: cfg.global.into(),
            machines,
            connections,
        })
    }
}
