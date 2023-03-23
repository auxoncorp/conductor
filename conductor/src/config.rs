use crate::types::{
    ConnectionKind, ConnectionName, EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths,
    InterfaceName, MachineName, ProviderKind, SystemName,
};
use conductor_config::{
    ConnectorPropertiesError, DockerMachineProvider, GpioConnectorProperties,
    NetworkConnectorProperties, QemuMachineProvider, RenodeMachineProvider,
    UartConnectorProperties,
};
use derive_more::Display;
use derive_more::From;
use std::{
    collections::BTreeSet,
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
    #[error("Machine '{_0}' does not have a provider specified")]
    NoMachineProvider(MachineName),
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
    //pub worlds: Vec<World>,
    pub machines: Vec<Machine>,
    pub connections: BTreeSet<Connection>,
    // TODO
    //pub storages: Vec<Storage>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Global {
    pub name: SystemName,
    pub environment_variables: EnvironmentVariableKeyValuePairs,
}

// TODO
// pub struct World {
// pub enum WorldProvider {
// pub struct GazeboWorldProvider {

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Machine {
    pub base: BaseMachine,
    pub provider: MachineProvider,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct BaseMachine {
    pub name: MachineName,
    pub bin: PathBuf,
    pub environment_variables: EnvironmentVariableKeyValuePairs,
    pub assets: HostToGuestAssetPaths,
    pub connectors: Vec<MachineConnector>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum MachineProvider {
    Renode(RenodeMachineProvider),
    Qemu(QemuMachineProvider),
    Docker(DockerMachineProvider),
}

impl MachineProvider {
    pub fn kind(&self) -> ProviderKind {
        use MachineProvider::*;
        match self {
            Renode(_) => ProviderKind::Renode,
            Qemu(_) => ProviderKind::Qemu,
            Docker(_) => ProviderKind::Docker,
        }
    }
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}")]
pub enum Connection {
    #[display(fmt = "{}:{}", "self.kind()", "self.name()")]
    Uart(UartConnection),
    #[display(fmt = "{}:{}", "self.kind()", "self.name()")]
    Gpio(GpioConnection),
    #[display(fmt = "{}:{}", "self.kind()", "self.name()")]
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", name)]
pub struct UartConnection {
    pub name: ConnectionName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", name)]
pub struct GpioConnection {
    pub name: ConnectionName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", name)]
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
            environment_variables: value.environment_variables.into(),
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
        let provider = value
            .provider
            .ok_or_else(|| ConfigError::NoMachineProvider(name.clone()))?;
        let mut connectors = Vec::with_capacity(value.connectors.len());
        for c in value.connectors.into_iter() {
            let c = MachineConnector::try_from((c, connections))?;
            if connectors.contains(&c) {
                return Err(ConfigError::DupConnector(name, c.name));
            }
            connectors.push(c);
        }
        Ok(Self {
            base: BaseMachine {
                name,
                bin,
                environment_variables: value.environment_variables.into(),
                assets: value.assets.into(),
                connectors,
            },
            provider: provider.into(),
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
        // TODO - do semantic checks on props
        // GPIO can only specify src or dest pin, not both
        // UART can only have one kind of host integration
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

impl From<conductor_config::MachineProvider> for MachineProvider {
    fn from(value: conductor_config::MachineProvider) -> Self {
        match value {
            conductor_config::MachineProvider::Renode(p) => MachineProvider::Renode(p),
            conductor_config::MachineProvider::Qemu(p) => MachineProvider::Qemu(p),
            conductor_config::MachineProvider::Docker(p) => MachineProvider::Docker(p),
        }
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
        // TODO(jon@auxon.io)
        // basic top-level validation
        // names exist
        // canonical names or provider-specific canonicalization, renode doesn't like spaces
        // providers are provided
        // resolve and check connectors to their connections
        //
        // provider-specific checks
        // renode
        //   if more than one machine per renode instance
        //   check for conflicts (env var keys, CLI config/opts, etc)
        //   script mut excl with script path, same for plat desc and commands, etc
        //   empty platform desc
        // ...

        let cfg = conductor_config::Config::read(&config_path)?;
        let cfg_dir = config_path.as_ref().parent();

        let mut connections = BTreeSet::new();
        for c in cfg.connections.into_iter() {
            let c = Connection::try_from(c)?;
            if let Some(prev_con) = connections.replace(c) {
                return Err(ConfigError::DupConnection(prev_con.name().clone()).into());
            }
        }

        let mut machines: Vec<Machine> = Vec::with_capacity(cfg.machines.len());
        for m in cfg.machines.into_iter() {
            let mut m = Machine::try_from((m, &connections))?;
            let contains_name_already = machines
                .iter()
                .any(|known_m| known_m.base.name == m.base.name);
            if contains_name_already {
                return Err(ConfigError::DupMachine(m.base.name).into());
            }
            // Convert relative paths on the host to absolute, where possible
            if let Some(cfg_dir) = cfg_dir {
                if m.base.bin.is_relative() {
                    m.base.bin = cfg_dir.join(m.base.bin);
                }
                if !m.base.bin.exists() {
                    return Err(ConfigError::NonExistentMachineBin(
                        m.base.bin.clone(),
                        m.base.name,
                    )
                    .into());
                }

                let assets = m.base.assets.0.clone();
                m.base.assets.0.clear();
                for (mut host_asset, guest_asset) in assets.into_iter() {
                    if host_asset.is_relative() {
                        host_asset = cfg_dir.join(host_asset);
                    }
                    if !host_asset.exists() {
                        return Err(
                            ConfigError::NonExistentMachineAsset(host_asset, m.base.name).into(),
                        );
                    }
                    m.base.assets.0.insert(host_asset, guest_asset);
                }
            }
            machines.push(m);
        }

        Ok(Self {
            global: cfg.global.into(),
            machines,
            connections,
        })
    }
}
