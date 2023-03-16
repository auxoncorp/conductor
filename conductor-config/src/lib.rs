use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

pub const DEFAULT_CONFIG_FILE_NAME: &str = "conductor.toml";

#[derive(Debug, thiserror::Error)]
pub enum ConfigReadError {
    #[error("Error in configuration file {}", .path.display())]
    ConfigToml {
        path: PathBuf,
        #[source]
        error: Box<toml::de::Error>,
    },

    #[error("Ecountered an IO error while reading the configuration file")]
    Io(#[from] Box<io::Error>),
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    #[serde(flatten)]
    pub global: Global,

    #[serde(alias = "simulator", skip_serializing_if = "Vec::is_empty")]
    pub simulators: Vec<Simulator>,

    #[serde(alias = "machine", skip_serializing_if = "Vec::is_empty")]
    pub machines: Vec<Machine>,

    #[serde(alias = "connection", skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<Connection>,

    #[serde(alias = "storage", skip_serializing_if = "Vec::is_empty")]
    pub storages: Vec<Storage>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Global {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Simulator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<SimulatorBackend>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SimulatorBackend {
    Gazebo(GazeboSimulatorBackend),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GazeboSimulatorBackend {
    pub world_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Machine {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environment_variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub assets: Vec<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<MachineBackend>,
    #[serde(alias = "connector", skip_serializing_if = "Vec::is_empty")]
    pub connectors: Vec<MachineConnector>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MachineBackend {
    Renode(RenodeMachineBackend),
    Qemu(QemuMachineBackend),
    Docker(DockerMachineBackend),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RenodeMachineBackend {
    #[serde(flatten)]
    pub context: toml::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct QemuMachineBackend {
    #[serde(flatten)]
    pub context: toml::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DockerMachineBackend {
    #[serde(flatten)]
    pub context: toml::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MachineConnector {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(flatten)]
    pub context: toml::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Connection {
    Uart(UartConnection),
    Gpio(GpioConnection),
    Network(NetworkConnection),
    WirelessNetwork(WirelessNetworkConnection),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UartConnection {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GpioConnection {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConnection {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WirelessNetworkConnection {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Storage {
    Virtio(VirtioStorage),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VirtioStorage {
    pub name: String,
    pub image: PathBuf,
}

impl Config {
    pub fn read<P: AsRef<Path>>(config_path: P) -> Result<Self, ConfigReadError> {
        let content = fs::read_to_string(&config_path).map_err(Box::new)?;
        Self::from_str(&content).map_err(|e| ConfigReadError::ConfigToml {
            path: config_path.as_ref().to_owned(),
            error: Box::new(e),
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
        [environment-variables]
        SOME_VAR = 'SOME_VAL'
        SOME_VAR2 = 'SOME_VAL2'

        [[simulator]]
        name = 'a world'
            [simulator.backend.gazebo]
            world-path = 'path/to/my.sdf'
            config-path = 'path/to/gz.conf'
            plugins-path = 'path/to/plugins'
            headless = false
            partition = 'my-sim-partition'

        [[machine]]
        name = "foo"
        bin = 'path/to/foo-firmware'
        assets = ['path/to/some/dir']
            [machine.environment-variables]
            M0_VAR = 'M0_VAL'
            M1_VAR = 'M1_VAL'

            [machine.backend.docker]
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
            [machine.backend.qemu]
            machine = 'mps2-an385'
            cpu = 'cortex-m3'
            memory = '16M'
            no-graphic = true
            [machine.backend.qemu.qmp]
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
            [machine.backend.renode]
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
        name = "barbiz-bt"
        type = "wireless-network"
        medium = "ble"

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
        assert_eq!(cfg.simulators.len(), 1);
        assert_eq!(cfg.machines.len(), 3);
    }
}
