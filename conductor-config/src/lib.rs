use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

pub use connector_properties::{
    ConnectorPropertiesError, GpioConnectorProperties, NetworkConnectorProperties,
    UartConnectorProperties,
};
pub use container::ContainerMachineProvider;
pub use gazebo::GazeboWorldProvider;
pub use qemu::{QemuMachineProtocolConfig, QemuMachineProvider};
pub use renode::{RenodeCliConfig, RenodeMachineProvider, RenodeScriptConfig};

mod connector_properties;
mod container;
mod gazebo;
mod qemu;
mod renode;

pub const DEFAULT_CONFIG_FILE_NAME: &str = "conductor.toml";
pub const DEFAULT_SYSTEM_NAME: &str = "default-system";

/// Searches for the `conductor.toml` file in the current working directory and above
pub fn find_config_file() -> Result<PathBuf, ConfigReadError> {
    use std::env;
    let mut wd = env::current_dir()?;

    while wd.as_os_str() != "/" {
        wd.push(DEFAULT_CONFIG_FILE_NAME);

        if wd.is_file() {
            return Ok(wd);
        }

        wd.pop();
        wd.pop();
    }

    Err(ConfigReadError::SearchFailed {
        search_path: env::current_dir()?,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigReadError {
    #[error("Error in configuration file {}", .path.display())]
    ConfigToml {
        path: PathBuf,
        #[source]
        error: toml::de::Error,
    },

    #[error("Configuration file not found in {} or any parent directory", .search_path.display())]
    SearchFailed { search_path: PathBuf },

    #[error("Ecountered an IO error while reading the configuration file")]
    Io(#[from] io::Error),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    pub global: Global,

    #[serde(alias = "world", skip_serializing_if = "Vec::is_empty")]
    pub worlds: Vec<World>,

    #[serde(alias = "machine", skip_serializing_if = "Vec::is_empty")]
    pub machines: Vec<Machine>,

    #[serde(alias = "connection", skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<Connection>,

    #[serde(alias = "storage", skip_serializing_if = "Vec::is_empty")]
    pub storages: Vec<Storage>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Global {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xauthority: Option<PathBuf>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct World {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<PathBuf, PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<WorldProvider>,
    #[serde(alias = "connector", skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<WorldConnector>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorldProvider {
    Gazebo(GazeboWorldProvider),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WorldConnector {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Machine {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<PathBuf>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<PathBuf, PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<MachineProvider>,
    #[serde(alias = "connector", skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<MachineConnector>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MachineProvider {
    Renode(RenodeMachineProvider),
    Qemu(QemuMachineProvider),
    Container(ContainerMachineProvider),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MachineConnector {
    pub name: String,
    pub interface: String,
    #[serde(flatten)]
    pub(crate) context: toml::Value,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Connection {
    Uart(UartConnection),
    Gpio(GpioConnection),
    Network(NetworkConnection),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UartConnection {
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GpioConnection {
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConnection {
    pub name: String,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Storage {
    Virtio(VirtioStorage),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VirtioStorage {
    pub name: String,
    pub image: PathBuf,
}

impl Config {
    pub fn read<P: AsRef<Path>>(config_path: P) -> Result<Self, ConfigReadError> {
        let content = fs::read_to_string(&config_path)?;
        Self::from_str(&content).map_err(|e| ConfigReadError::ConfigToml {
            path: config_path.as_ref().to_owned(),
            error: e,
        })
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    const FULL_TOML: &str = indoc! {r#"
        name = 'my system'
        display = ':0'
        xauthority = '/not/home/.Xauthority'

        [environment-variables]
        SOME_VAR = 'SOME_VAL'
        SOME_VAR2 = 'SOME_VAL2'

        [[world]]
        name = 'a world'
            [world.provider.gazebo]
            world-path = 'path/to/my.sdf'
            config-path = 'path/to/gz.conf'
            plugins-path = 'path/to/plugins'
            headless = false
            partition = 'my-sim-partition'

            [[world.connector]]
            name = "foobiz"

        [[machine]]
        name = "foo"
        bin = 'path/to/foo-firmware'
            [machine.assets]
            'path/to/some/host/dir' = 'path/on/guest'

            [machine.environment-variables]
            M0_VAR = 'M0_VAL'
            M1_VAR = 'M1_VAL'

            [machine.provider.container]
            foo = "bar"

            [[machine.connector]]
            name = "foobar"
            interface = "sysbus.uart1"
            special-thing = 'foo'

            [[machine.connector]]
            name = "foobiz"
            interface = "sysbus.ethernet"
            this-one = 1

        [[machine]]
        name = "bar"
        bin = 'path/to/bar-firmware.bin'
            [machine.environment-variables]
            M0_VAR = 'M0_VAL_BAR'

            [machine.provider.qemu]
            machine = 'mps2-an385'
            cpu = 'cortex-m3'
            memory = '16M'
            no-graphic = true
            [machine.provider.qemu.qmp]
            port = 4444
            wait = false
            server = true

            [[machine.connector]]
            name = "foobar"
            interface = "serial4"
            more-special-thing = 'bar'

            [[machine.connector]]
            name = "barbiz-bt"
            interface = "net4"
            macaddr = '52:54:00:12:34:AD'
            model = 'lan9118'

        [[machine]]
        name = "biz"
        bin = 'path/to/biz-firmware.elf'
            [machine.provider.renode]
            disable-xwt = true
            console = true
            platform-descriptions = [
                '@platforms/boards/stm32f4_discovery-kit.repl',
                'my/local/dev_board.repl',
                '''
                phy3: Network.EthernetPhysicalLayer @ ethernet 3
                    Id1: 0x0000
                    Id2: 0x0000
                ''',
            ]
            commands = [
                'logLevel -1 i2c2',
                'ethernet.phy1 Id1 0',
                '''
                python "import _random"
                python "rand = _random.Random()"

                $id1 = `python "print rand.getrandbits(32)"`
                $id2 = `python "print rand.getrandbits(32)"`
                $id3 = `python "print rand.getrandbits(32)"`
                ''',
            ]
            reset-macro = '''
            sysbus LoadELF $bin
            sysbus WriteDoubleWord 0x1FFF7A10 $id1
            sysbus WriteDoubleWord 0x1FFF7A14 $id2
            sysbus WriteDoubleWord 0x1FFF7A18 $id3
            '''

            [[machine.connector]]
            name = "barbiz-bt"
            interface = "net0"

            [[machine.connector]]
            name = "foobiz"
            interface = "net2"

        [[connection]]
        name = "foobar"
        type = "uart"

        [[connection]]
        name = "barbiz"
        type = "gpio"

        [[connection]]
        name = "foobiz"
        type = "network"
        host-tap = "tap0"

        [[storage]]
        name = "my-img"
        type = "virtio"
        image = 'path/to/my.img'
    "#};

    #[test]
    fn read_config_file() {
        let td = tempfile::tempdir().unwrap();
        let cfg_path = td.path().join(DEFAULT_CONFIG_FILE_NAME);
        fs::write(&cfg_path, FULL_TOML).unwrap();
        let cfg = Config::read(&cfg_path).unwrap();

        assert_eq!(cfg.global.environment_variables.len(), 2);
        assert_eq!(cfg.worlds.len(), 1);
        assert_eq!(cfg.machines.len(), 3);
    }
}
