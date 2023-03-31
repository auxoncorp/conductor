use crate::{
    config::{Connection, ConnectorProperties},
    provider::renode::RenodeMachine,
    types::{ConnectionKind, ConnectionName, InterfaceName},
};
use std::{io, path::Path};

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
    ) -> io::Result<()> {
        for c in connections.iter() {
            self.gen_connection_create(c.name(), c.kind())?;
        }
        writeln!(self.w)?;

        for m in machines.iter() {
            self.gen_machine_create(m)?;
            self.gen_machine_set(m)?;
            writeln!(self.w, "$bin = {}", resc_path(&m.guest_bin()))?;
            for repl in m.provider.resc.platform_descriptions.iter() {
                // TODO - we'll need to support all the variants of platform descriptions
                writeln!(self.w, "machine LoadPlatformDescription @{repl}")?;
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

    fn gen_connection_create(
        &mut self,
        name: &ConnectionName,
        kind: ConnectionKind,
    ) -> io::Result<()> {
        use ConnectionKind::*;
        let op = match kind {
            Uart => "CreateUARTHub",
            Gpio => "CreateGPIOConnector",
            Network => "CreateSwitch",
        };
        writeln!(self.w, "emulation {op} \"{name}\"")
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
        RenodeScriptConfig,
    };
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use std::{path::PathBuf, str};

    const RESC: &str = indoc! {r#"
        emulation CreateUARTHub "foo-uart"
        emulation CreateGPIOConnector "foo-gpio"
        emulation CreateSwitch "foo-net"

        mach create "my-m0"
        mach set "my-m0"
        $bin = @/conductor_resources/my-m0/m0.bin
        machine LoadPlatformDescription @platforms/cpus/stm32f429.repl
        connector Connect sysbus.usart0 "foo-uart"
        connector Connect sysbus.gpioPortA "foo-gpio"
        foo-gpio SelectSourcePin sysbus.gpioPortA 2
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
                    bin: PathBuf::from("path/to/m0.bin"),
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
            },
            RenodeMachine {
                guest_bin_shared: false,
                base: BaseMachine {
                    name: MachineName::new_canonicalize("my-m1").unwrap(),
                    bin: PathBuf::from("path/to/m1.bin"),
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
            },
        ]
    }

    #[test]
    fn renode_script_generation() {
        let mut resc = Vec::new();
        let machines = machines();
        let connections = connections();
        RenodeScriptGen::new(&mut resc)
            .generate(&machines, &connections)
            .unwrap();
        let out = str::from_utf8(&resc).unwrap();
        assert_eq!(out, RESC);
    }
}
