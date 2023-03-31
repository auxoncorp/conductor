use crate::{ComponentGraph, Config, WorldOrMachineComponent};
use anyhow::{bail, Result};
use std::borrow::Cow;
use std::path::{Path, PathBuf};

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
                            MachineProvider::Container(ContainerMachineProvider {
                                image: cont_prov_cfg.image.clone(),
                                containerfile: cont_prov_cfg.containerfile.clone(),
                                context: cont_prov_cfg.context.clone(),
                            })
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
    image: Option<String>,
    containerfile: Option<PathBuf>,
    context: Option<PathBuf>,
}

impl ContainerMachineProvider {
    pub async fn build(&mut self) -> Result<()> {
        match (&self.image, &self.containerfile, &self.context) {
            (Some(image_name), None, None) => {
                crate::containers::build_image_from_name(image_name).await
            }
            (Some(image_name), Some(containerfile_path), None) => {
                crate::containers::build_image_from_containerfile(
                    image_name,
                    containerfile_path.to_path_buf(),
                )
                .await
            }
            (Some(_), Some(_containerfile_path), Some(_)) => {
                todo!("build named container file with seperate context")
            }
            (Some(_), None, Some(_)) => todo!("build named container from context"),
            (None, Some(_containerfile_path), None) => todo!("build unnamed containerfile"),
            (None, Some(_containerfile_path), Some(_)) => {
                todo!("build unnamed container from containerfile with seperate context")
            }
            (None, None, Some(_)) => {
                todo!("build unnamed container from context")
            }
            (None, None, None) => bail!("none of `image`, `containerfile`, or `context` provided"),
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let image = match (&self.image, &self.containerfile, &self.context) {
            (Some(image_name), None, None) => Cow::Borrowed(image_name),
            (Some(image_name), Some(_), _) | (Some(image_name), _, Some(_)) => {
                Cow::Owned(format!("conductor/{image_name}"))
            }
            (None, _, _) => todo!("figure out starting images without names"),
        };
        crate::containers::start_container_from_image(&image).await
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
                    image: Some("docker.io/ubuntu:latest".to_string()),
                    containerfile: None,
                    context: None,
                }),
            }],
        };

        system.build().await?;

        system.start().await?;

        Ok(())
    }
}
