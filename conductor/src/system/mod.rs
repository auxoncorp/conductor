use crate::{ComponentGraph, Config, WorldOrMachineComponent};
use anyhow::Result;
use std::path::Path;

use crate::containers::Container;
use crate::types::MachineName;

pub struct System {
    config: Config,
    machines: Vec<Machine>,
}

impl System {
    #[allow(clippy::let_and_return)]
    pub fn from_config(config: Config) -> Self {
        let system = System {
            config,
            machines: Vec::new(),
        };

        // TODO(jon@auxon.io) disabled so I could run without tripping the todo!'s
        //system.init_self();

        system
    }

    pub fn try_from_config_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = Config::read(path)?;
        Ok(Self::from_config(config))
    }

    pub fn try_from_working_directory() -> Result<Self> {
        let config_path = conductor_config::find_config_file()?;
        let config = Config::read(config_path)?;
        Ok(Self::from_config(config))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn components(&self) -> Vec<WorldOrMachineComponent> {
        self.config
            .worlds
            .iter()
            .cloned()
            .map(WorldOrMachineComponent::from)
            .chain(
                self.config
                    .machines
                    .iter()
                    .cloned()
                    .map(WorldOrMachineComponent::from),
            )
            .collect()
    }

    pub fn graph(&self) -> Result<ComponentGraph<WorldOrMachineComponent>> {
        let components = self.components();
        let connections = self.config.connections.clone();
        let g = ComponentGraph::new(components, connections)?;
        Ok(g)
    }

    // TODO.pb: This should handle conversion of all of `Config` and `System` shouldn't have a raw
    // `Config` at all. This should also be inline in `from_config` before the creation of a
    // `System`.
    pub fn init_self(&mut self) {
        for component in self.components() {
            match component {
                WorldOrMachineComponent::Machine(machine) => {
                    println!("machine: {machine:?}");
                    let provider = match &machine.provider {
                        crate::config::MachineProvider::Container(cont_prov_cfg) => {
                            // This is not great, not sure which way I want to fix this yet.
                            let mut container = Container::new();
                            if let Some(ref image) = cont_prov_cfg.image {
                                container.set_image(image);
                            };
                            if let Some(ref containerfile) = cont_prov_cfg.containerfile {
                                container.set_containerfile(containerfile);
                            };
                            if let Some(ref context) = cont_prov_cfg.context {
                                container.set_context(context);
                            };
                            if !machine.base.assets.is_empty() {
                                let mounts = machine.base.assets.as_ref().iter().map(|asset| {
                                    (asset.0.to_str().unwrap(), asset.1.to_str().unwrap())
                                });
                                container.set_mounts(mounts);
                            };
                            // TODO: get this from bin once bin is optional and plumbed through
                            if let Some(ref cmd) = None::<Vec<String>> {
                                container.set_cmd(cmd);
                            };
                            MachineProvider::Container(ContainerMachineProvider { container })
                        }
                        _ => todo!("provider not yet supported"),
                    };

                    let machine = Machine {
                        _name: machine.base.name.clone(),
                        provider,
                    };

                    self.machines.push(machine);
                }
                WorldOrMachineComponent::World(_world) => {
                    todo!("build worlds?");
                }
            }
        }
    }

    pub async fn build(&mut self) -> Result<()> {
        for machine in &mut self.machines {
            machine.build().await?;
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        for machine in &mut self.machines {
            machine.start().await?;
        }

        Ok(())
    }
}

pub struct Machine {
    _name: MachineName,
    provider: MachineProvider,
}

impl Machine {
    pub async fn build(&mut self) -> Result<()> {
        self.provider.build().await
    }

    pub async fn start(&mut self) -> Result<()> {
        self.provider.start().await
    }
}

pub enum MachineProvider {
    Container(ContainerMachineProvider),
    Renode(RenodeMachineProvider),
    Qemu(QemuMachineProvider),
}

impl MachineProvider {
    pub async fn build(&mut self) -> Result<()> {
        match self {
            MachineProvider::Container(cont) => cont.build().await,
            _ => todo!("provider type not yet supported"),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        match self {
            MachineProvider::Container(cont) => cont.start().await,
            _ => todo!("provider type not yet supported"),
        }
    }
}

pub struct ContainerMachineProvider {
    container: Container,
}

impl ContainerMachineProvider {
    pub async fn build(&mut self) -> Result<()> {
        self.container.build().await
    }

    pub async fn start(&mut self) -> Result<()> {
        self.container.run().await
    }
}

pub struct RenodeMachineProvider {
    _container: ContainerMachineProvider,
}

pub struct QemuMachineProvider {
    _container: ContainerMachineProvider,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Global;
    use crate::types::SystemName;
    use std::collections::BTreeSet;

    #[test]
    fn get_system_from_config_path() -> Result<()> {
        System::try_from_config_path(
            "../test_resources/systems/single-container-machine/conductor.toml",
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn run_fake_system() -> Result<()> {
        let mut system = System {
            config: Config {
                global: Global {
                    name: SystemName::new_canonicalize("fake-system").unwrap(),
                    environment_variables: Default::default(),
                },
                machines: Vec::new(),
                connections: BTreeSet::new(),
                worlds: Vec::new(),
            },
            machines: vec![Machine {
                _name: MachineName::new_canonicalize("fake-machine").unwrap(),
                provider: MachineProvider::Container(ContainerMachineProvider {
                    container: Container::new().with_image("docker.io/ubuntu:latest"),
                }),
            }],
        };

        system.build().await?;

        system.start().await?;

        Ok(())
    }
}
