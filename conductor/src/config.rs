use crate::component::{Component, ComponentConnector};
use crate::types::{
    ComponentName, ConnectionKind, ConnectionName, EnvironmentVariableKeyValuePairs,
    HostToGuestAssetPaths, InterfaceName, MachineName, ProviderKind, SystemName, WorldName,
};
use conductor_config::{
    ConnectorPropertiesError, ContainerMachineProvider, GazeboWorldProvider,
    GpioConnectorProperties, NetworkConnectorProperties, QemuMachineProvider,
    RenodeMachineProvider, UartConnectorProperties,
};
use derive_more::{Display, From};
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
    DupMachineConnector(MachineName, ConnectionName),
    #[error("World '{_0}' has a invalid duplicate connector '{_1}'")]
    DupWorldConnector(WorldName, ConnectionName),
    #[error("A machine connector references a connection '{_0}' that isn't defined")]
    MissingMachineConnectorConnection(ConnectionName),
    #[error("A world connector references a connection '{_0}' that isn't defined")]
    MissingWorldConnectorConnection(ConnectionName),
    #[error("A machine must have a name")]
    EmptyMachineName,
    #[error("A world must have a name")]
    EmptyWorldName,
    #[error("The host binary '{_0:?}' for machine '{_1}' does not exist")]
    NonExistentMachineBin(PathBuf, MachineName),
    #[error("The host asset '{_0:?}' for machine '{_1}' does not exist")]
    NonExistentMachineAsset(PathBuf, MachineName),
    #[error("The host asset '{_0:?}' for world '{_1}' does not exist")]
    NonExistentWorldAsset(PathBuf, WorldName),
    #[error("Machine '{_0}' does not have a provider specified")]
    NoMachineProvider(MachineName),
    #[error("World '{_0}' does not have a provider specified")]
    NoWorldProvider(WorldName),
    #[error("Machine '{_0}' does not have a bin path specified")]
    NoMachineBin(MachineName),
    #[error("Found duplicate machines with name '{_0}'")]
    DupMachine(MachineName),
    #[error("Found duplicate worlds with name '{_0}'")]
    DupWorld(WorldName),
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
    pub worlds: Vec<World>,
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{}] {}", "self.provider()", "self.base.name")]
pub struct World {
    pub base: BaseWorld,
    pub provider: WorldProvider,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct BaseWorld {
    pub name: WorldName,
    pub environment_variables: EnvironmentVariableKeyValuePairs,
    pub assets: HostToGuestAssetPaths,
    pub connectors: Vec<WorldConnector>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub enum WorldProvider {
    Gazebo(GazeboWorldProvider),
}

impl WorldProvider {
    pub fn kind(&self) -> ProviderKind {
        use WorldProvider::*;
        match self {
            Gazebo(_) => ProviderKind::Gazebo,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WorldConnector {
    pub name: ConnectionName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{}] {}", "self.provider()", "self.base.name")]
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub enum MachineProvider {
    Renode(RenodeMachineProvider),
    Qemu(QemuMachineProvider),
    Container(ContainerMachineProvider),
}

impl MachineProvider {
    pub fn kind(&self) -> ProviderKind {
        use MachineProvider::*;
        match self {
            Renode(_) => ProviderKind::Renode,
            Qemu(_) => ProviderKind::Qemu,
            Container(_) => ProviderKind::Container,
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

impl ConnectorProperties {
    /// Returns Some(true) if this is the source/initiator side of an asymmetrical connection.
    /// Returns None for symmetrical connections.
    pub fn is_asymmetrical_initiator(&self) -> Option<bool> {
        use ConnectorProperties::*;
        match self {
            Uart(_p) => None,
            Gpio(p) => Some(p.source_pin.is_some()),
            Network(_p) => None,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From, Display)]
#[display(fmt = "{}")]
pub enum Connection {
    #[display(fmt = "<{}> {}", "self.kind()", "self.name()")]
    Uart(UartConnection),
    #[display(fmt = "<{}> {}", "self.kind()", "self.name()")]
    Gpio(GpioConnection),
    #[display(fmt = "<{}> {}", "self.kind()", "self.name()")]
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

impl TryFrom<(conductor_config::World, &BTreeSet<Connection>)> for World {
    type Error = ConfigError;

    fn try_from(
        values: (conductor_config::World, &BTreeSet<Connection>),
    ) -> Result<Self, Self::Error> {
        let (value, connections) = values;
        let name = value
            .name
            .as_ref()
            .and_then(WorldName::new)
            .ok_or(ConfigError::EmptyWorldName)?;
        let provider = value
            .provider
            .ok_or_else(|| ConfigError::NoWorldProvider(name.clone()))?;
        let mut connectors = Vec::with_capacity(value.connectors.len());
        for c in value.connectors.into_iter() {
            let c = WorldConnector::try_from((c, connections))?;
            if connectors.contains(&c) {
                return Err(ConfigError::DupWorldConnector(name, c.name));
            }
            connectors.push(c);
        }
        Ok(Self {
            base: BaseWorld {
                name,
                environment_variables: value.environment_variables.into(),
                assets: value.assets.into(),
                connectors,
            },
            provider: provider.into(),
        })
    }
}

impl TryFrom<(conductor_config::WorldConnector, &BTreeSet<Connection>)> for WorldConnector {
    type Error = ConfigError;

    fn try_from(
        values: (conductor_config::WorldConnector, &BTreeSet<Connection>),
    ) -> Result<Self, Self::Error> {
        let (value, _connections) = values;
        let name = ConnectionName::new(value.name).ok_or(ConfigError::EmptyConnectorName)?;
        Ok(Self { name })
    }
}

impl From<conductor_config::WorldProvider> for WorldProvider {
    fn from(value: conductor_config::WorldProvider) -> Self {
        match value {
            conductor_config::WorldProvider::Gazebo(p) => WorldProvider::Gazebo(p),
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
                return Err(ConfigError::DupMachineConnector(name, c.name));
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
            .ok_or_else(|| ConfigError::MissingMachineConnectorConnection(name.clone()))?;
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
            conductor_config::MachineProvider::Container(p) => MachineProvider::Container(p),
        }
    }
}

impl TryFrom<conductor_config::Connection> for Connection {
    type Error = ConfigError;

    fn try_from(value: conductor_config::Connection) -> Result<Self, Self::Error> {
        Ok(match value {
            conductor_config::Connection::Uart(c) => UartConnection::try_from(c)?.into(),
            conductor_config::Connection::Gpio(c) => GpioConnection::try_from(c)?.into(),
            conductor_config::Connection::Network(c) => NetworkConnection::try_from(c)?.into(),
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

impl Component for World {
    fn name(&self) -> ComponentName {
        self.base.name.clone().into()
    }

    fn provider(&self) -> ProviderKind {
        self.provider.kind()
    }

    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs {
        &self.base.environment_variables
    }

    fn assets(&self) -> &HostToGuestAssetPaths {
        &self.base.assets
    }

    fn connectors(&self) -> Vec<ComponentConnector> {
        self.base
            .connectors
            .iter()
            .cloned()
            .map(ComponentConnector::from)
            .collect()
    }
}

impl Component for Machine {
    fn name(&self) -> ComponentName {
        self.base.name.clone().into()
    }

    fn provider(&self) -> ProviderKind {
        self.provider.kind()
    }

    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs {
        &self.base.environment_variables
    }

    fn assets(&self) -> &HostToGuestAssetPaths {
        &self.base.assets
    }

    fn connectors(&self) -> Vec<ComponentConnector> {
        self.base
            .connectors
            .iter()
            .cloned()
            .map(ComponentConnector::from)
            .collect()
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
        //
        // gazebo
        //   other fields need path exist checks
        //
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

        let mut worlds: Vec<World> = Vec::with_capacity(cfg.worlds.len());
        for w in cfg.worlds.into_iter() {
            let mut w = World::try_from((w, &connections))?;
            let contains_name_already = worlds
                .iter()
                .any(|known_w| known_w.base.name == w.base.name);
            if contains_name_already {
                return Err(ConfigError::DupWorld(w.base.name).into());
            }
            // Convert relative paths on the host to absolute, where possible
            if let Some(cfg_dir) = cfg_dir {
                let assets = w.base.assets.0.clone();
                w.base.assets.0.clear();
                for (mut host_asset, guest_asset) in assets.into_iter() {
                    if host_asset.is_relative() {
                        host_asset = cfg_dir.join(host_asset);
                    }
                    if !host_asset.exists() {
                        return Err(
                            ConfigError::NonExistentWorldAsset(host_asset, w.base.name).into()
                        );
                    }
                    w.base.assets.0.insert(host_asset, guest_asset);
                }
            }
            worlds.push(w);
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

                if let MachineProvider::Container(ref mut cmp) = m.provider {
                    if let Some(ref mut containerfile) = cmp.containerfile {
                        if containerfile.is_relative() {
                            *containerfile = cfg_dir.join(&containerfile)
                        }
                    }
                    if let Some(ref mut context) = cmp.context {
                        if context.is_relative() {
                            *context = cfg_dir.join(&context)
                        }
                    }
                }
            }
            machines.push(m);
        }

        Ok(Self {
            global: cfg.global.into(),
            worlds,
            machines,
            connections,
        })
    }
}
