use crate::{
    config::{Connection, ConnectorProperties},
    provider::{
        guest_component_resource_path,
        renode::{PlatformDescription, RenodeMachine, TapDevice},
    },
    types::{ConnectionKind, ConnectionName, InterfaceName},
};
use std::{collections::BTreeMap, io, path::Path};

pub struct RenodeScriptGen<'a, T: io::Write> {
    w: &'a mut T,
}

impl<'a, T: io::Write> RenodeScriptGen<'a, T> {
    pub fn new(writer: &'a mut T) -> Self {
        Self { w: writer }
    }

    // TODO
    // handle script/script-path or rm them from the config type
    // platform_descriptions can be path, local path, etc
    pub fn generate(
        mut self,
        machines: &[RenodeMachine],
        connections: &[Connection],
        tap_devices: &BTreeMap<ConnectionName, TapDevice>,
    ) -> io::Result<()> {
        let connections: Vec<RenodeConnection> = connections
            .iter()
            .cloned()
            .map(|c| {
                if let Some(uart_connector) = machines
                    .iter()
                    .flat_map(|m| m.base.connectors.iter())
                    .filter(|mc| &mc.name == c.name() && mc.properties.is_guest_to_host())
                    .find_map(|mc| match &mc.properties {
                        ConnectorProperties::Uart(p) => p.port.map(|port| {
                            (mc.interface.clone(), port, p.emit_config.unwrap_or(true))
                        }),
                        _ => None,
                    })
                {
                    // Guest-to-host connections only have a single machine connector
                    RenodeConnection::GuestToHostUartSocketTerm(
                        c,
                        uart_connector.0,
                        uart_connector.1,
                        uart_connector.2,
                    )
                } else {
                    RenodeConnection::GuestToGuest(c)
                }
            })
            .collect();

        for c in connections.iter() {
            self.gen_connection_create(c)?;
        }

        if !connections.is_empty() {
            writeln!(self.w)?;
        }

        for (idx, (switch_name, tap_device)) in tap_devices.iter().enumerate() {
            writeln!(self.w, "emulation CreateTap \"{tap_device}\" \"tap{idx}\"")?;
            writeln!(self.w, "connector Connect host.tap{idx} \"{switch_name}\"")?;
        }
        if !tap_devices.is_empty() {
            writeln!(self.w)?;
        }

        for m in machines.iter() {
            self.gen_machine_create(m)?;
            self.gen_machine_set(m)?;
            writeln!(self.w, "$bin = {}", resc_path(&m.guest_bin()))?;
            for pd in m.platform_descriptions.iter() {
                // TODO - we'll need to support all the variants of platform descriptions
                let guest_repl_path = match pd {
                    PlatformDescription::ProvidedByRenode(repl) => repl.clone(),
                    PlatformDescription::LocalFile(file_name, _content) => {
                        guest_component_resource_path(&m.base.name).join(file_name)
                    }
                };
                writeln!(
                    self.w,
                    "machine LoadPlatformDescription @{}",
                    guest_repl_path.display()
                )?;
            }
            for c in m.base.connectors.iter() {
                let con_exists = connections.iter().any(|conn| conn.name() == &c.name);
                if !con_exists {
                    tracing::warn!(
                        machine = %m.base.name,
                        connection = %c.name,
                        "Generating a connector possibly without a connection"
                    );
                }

                self.gen_connector_connect(&c.name, &c.interface)?;
                self.gen_connector_properties(&c.name, &c.interface, &c.properties)?;
            }

            for cmd in m.provider.resc.commands.iter() {
                writeln!(self.w, "{cmd}")?;
            }
            let reset_macro = m
                .provider
                .resc
                .reset_macro
                .as_deref()
                .unwrap_or("sysbus LoadELF $bin");
            writeln!(self.w, "macro reset \"{reset_macro}\"")?;
            writeln!(self.w, "runMacro $reset")?;
            writeln!(self.w)?;
        }

        writeln!(self.w, "start")?;

        Ok(())
    }

    fn gen_machine_create(&mut self, m: &RenodeMachine) -> io::Result<()> {
        writeln!(self.w, "mach create \"{}\"", m.base.name)
    }

    fn gen_machine_set(&mut self, m: &RenodeMachine) -> io::Result<()> {
        writeln!(self.w, "mach set \"{}\"", m.base.name)
    }

    fn gen_connection_create(&mut self, connection: &RenodeConnection) -> io::Result<()> {
        use ConnectionKind::*;
        use RenodeConnection::*;

        match connection {
            GuestToHostUartSocketTerm(c, _iface, port, emit_cfg) => writeln!(
                self.w,
                "emulation CreateServerSocketTerminal {port} \"{name}\" {emit_cfg}",
                name = c.name()
            ),
            GuestToGuest(c) => {
                let op = match c.kind() {
                    Uart => "CreateUARTHub",
                    Gpio => "CreateGPIOConnector",
                    Network => "CreateSwitch",
                };
                writeln!(self.w, "emulation {op} \"{name}\"", name = c.name())
            }
        }
    }

    fn gen_connector_connect(
        &mut self,
        name: &ConnectionName,
        iface: &InterfaceName,
    ) -> io::Result<()> {
        writeln!(self.w, "connector Connect {iface} \"{name}\"")
    }

    fn gen_connector_properties(
        &mut self,
        name: &ConnectionName,
        iface: &InterfaceName,
        props: &ConnectorProperties,
    ) -> io::Result<()> {
        match props {
            ConnectorProperties::Uart(_p) => {
                // TODO: host integration props
            }
            ConnectorProperties::Gpio(p) => {
                if let Some(src_pin) = p.source_pin {
                    writeln!(self.w, "{name} SelectSourcePin {iface} {src_pin}")?;
                }
                if let Some(dst_pin) = p.destination_pin {
                    writeln!(self.w, "{name} SelectDestinationPin {iface} {dst_pin}")?;
                }
            }
            ConnectorProperties::Network(p) => {
                if let Some(promiscuous_mode) = p.promiscuous_mode {
                    match promiscuous_mode {
                        true => writeln!(self.w, "{name} EnablePromiscuousMode {iface}")?,
                        false => writeln!(self.w, "{name} DisablePromiscuousMode {iface}")?,
                    }
                }
            }
        }
        Ok(())
    }
}

type PortNumber = u16;
type EmitConfig = bool;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum RenodeConnection {
    GuestToHostUartSocketTerm(Connection, InterfaceName, PortNumber, EmitConfig),

    // TODO pty not supported yet, since it requires further device mapping
    // in the runtime container
    //GuestToHostUartPty(Connection, MachineConnector),

    // Normal conections
    GuestToGuest(Connection),
}

impl RenodeConnection {
    fn name(&self) -> &ConnectionName {
        use RenodeConnection::*;
        match self {
            GuestToHostUartSocketTerm(c, _, _, _) => c.name(),
            GuestToGuest(c) => c.name(),
        }
    }
}

/// Paths in renode scripts (.resc) use C# conventions
fn resc_path<P: AsRef<Path>>(p: P) -> String {
    format!("@{}", p.as_ref().display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            BaseMachine, GpioConnection, MachineConnector, NetworkConnection, UartConnection,
        },
        types::MachineName,
    };
    use conductor_config::{
        GpioConnectorProperties, NetworkConnectorProperties, RenodeMachineProvider,
        RenodeScriptConfig, UartConnectorProperties,
    };
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use std::{path::PathBuf, str};

    const RESC: &str = indoc! {r#"
        emulation CreateUARTHub "foo-uart"
        emulation CreateServerSocketTerminal 1234 "foo-uart-socket" false
        emulation CreateGPIOConnector "foo-gpio"
        emulation CreateSwitch "foo-net"

        emulation CreateTap "my_tap" "tap0"
        connector Connect host.tap0 "foo-net"

        mach create "my-m0"
        mach set "my-m0"
        $bin = @/conductor_resources/my-m0/m0.bin
        machine LoadPlatformDescription @platforms/cpus/stm32f429.repl
        connector Connect sysbus.usart0 "foo-uart"
        connector Connect sysbus.gpioPortA "foo-gpio"
        foo-gpio SelectSourcePin sysbus.gpioPortA 2
        connector Connect sysbus.usart2 "foo-uart-socket"
        connector Connect sysbus.ethernet "foo-net"
        cpu PerformanceInMips 1
        macro reset "sysbus LoadHEX $bin"
        runMacro $reset

        mach create "my-m1"
        mach set "my-m1"
        $bin = @/conductor_resources/my-m1/m1.bin
        machine LoadPlatformDescription @platforms/cpus/stm32f411.repl
        connector Connect sysbus.usart3 "foo-uart"
        connector Connect sysbus.gpioPortB "foo-gpio"
        foo-gpio SelectDestinationPin sysbus.gpioPortB 4
        connector Connect sysbus.eth2 "foo-net"
        foo-net EnablePromiscuousMode sysbus.eth2
        macro reset "sysbus LoadELF $bin"
        runMacro $reset

        start
    "#};

    fn connections() -> Vec<Connection> {
        vec![
            Connection::Uart(UartConnection {
                name: ConnectionName::new_canonicalize("foo-uart").unwrap(),
            }),
            Connection::Uart(UartConnection {
                name: ConnectionName::new_canonicalize("foo-uart-socket").unwrap(),
            }),
            Connection::Gpio(GpioConnection {
                name: ConnectionName::new_canonicalize("foo-gpio").unwrap(),
            }),
            Connection::Network(NetworkConnection {
                name: ConnectionName::new_canonicalize("foo-net").unwrap(),
            }),
        ]
    }

    fn machines() -> Vec<RenodeMachine> {
        vec![
            RenodeMachine {
                guest_bin_shared: false,
                base: BaseMachine {
                    name: MachineName::new_canonicalize("my-m0").unwrap(),
                    bin: PathBuf::from("path/to/m0.bin").into(),
                    environment_variables: Default::default(),
                    assets: Default::default(),
                    connectors: vec![
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-uart").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.usart0").unwrap(),
                            properties: ConnectorProperties::Uart(Default::default()),
                        },
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-gpio").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.gpioPortA").unwrap(),
                            properties: ConnectorProperties::Gpio(GpioConnectorProperties {
                                source_pin: Some(2),
                                ..Default::default()
                            }),
                        },
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-uart-socket").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.usart2").unwrap(),
                            properties: ConnectorProperties::Uart(UartConnectorProperties {
                                port: Some(1234),
                                emit_config: Some(false),
                                ..Default::default()
                            }),
                        },
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-net").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.ethernet").unwrap(),
                            properties: ConnectorProperties::Network(Default::default()),
                        },
                    ],
                },
                provider: RenodeMachineProvider {
                    cli: Default::default(),
                    resc: RenodeScriptConfig {
                        platform_descriptions: vec!["platforms/cpus/stm32f429.repl".to_string()],
                        commands: vec!["cpu PerformanceInMips 1".to_string()],
                        reset_macro: Some("sysbus LoadHEX $bin".to_string()),
                        ..Default::default()
                    },
                },
                platform_descriptions: vec![PlatformDescription::ProvidedByRenode(PathBuf::from(
                    "platforms/cpus/stm32f429.repl",
                ))],
                executable: PathBuf::from("path/to/m0.bin").into(),
                tap_devices: Default::default(),
            },
            RenodeMachine {
                guest_bin_shared: false,
                base: BaseMachine {
                    name: MachineName::new_canonicalize("my-m1").unwrap(),
                    bin: PathBuf::from("path/to/m1.bin").into(),
                    environment_variables: Default::default(),
                    assets: Default::default(),
                    connectors: vec![
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-uart").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.usart3").unwrap(),
                            properties: ConnectorProperties::Uart(Default::default()),
                        },
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-gpio").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.gpioPortB").unwrap(),
                            properties: ConnectorProperties::Gpio(GpioConnectorProperties {
                                destination_pin: Some(4),
                                ..Default::default()
                            }),
                        },
                        MachineConnector {
                            name: ConnectionName::new_canonicalize("foo-net").unwrap(),
                            interface: InterfaceName::new_canonicalize("sysbus.eth2").unwrap(),
                            properties: ConnectorProperties::Network(NetworkConnectorProperties {
                                promiscuous_mode: Some(true),
                            }),
                        },
                    ],
                },
                provider: RenodeMachineProvider {
                    cli: Default::default(),
                    resc: RenodeScriptConfig {
                        platform_descriptions: vec!["platforms/cpus/stm32f411.repl".to_string()],
                        ..Default::default()
                    },
                },
                platform_descriptions: vec![PlatformDescription::ProvidedByRenode(PathBuf::from(
                    "platforms/cpus/stm32f411.repl",
                ))],
                executable: PathBuf::from("path/to/m1.bin").into(),
                tap_devices: Default::default(),
            },
        ]
    }

    #[test]
    fn renode_script_generation() {
        let mut resc = Vec::new();
        let machines = machines();
        let connections = connections();
        let tap_devices = BTreeMap::from_iter(std::iter::once((
            ConnectionName::new_canonicalize("foo-net").unwrap(),
            "my_tap".to_string(),
        )));
        RenodeScriptGen::new(&mut resc)
            .generate(&machines, &connections, &tap_devices)
            .unwrap();
        let out = str::from_utf8(&resc).unwrap();
        assert_eq!(out, RESC);
    }
}
