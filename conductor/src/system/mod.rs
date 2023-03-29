use crate::{Config, WorldOrMachineComponent};
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

use crate::types::MachineName;

pub struct System {
    config: Config,
    machines: Vec<Machine>,
}

impl System {
    pub fn from_config(config: Config) -> Self {
        let mut system = System {
            config,
            machines: Vec::new(),
        };

        system.init_self();

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

    // TODO.pb: This should handle conversion of all of `Config` and `System` shouldn't have a raw
    // `Config` at all. This should also be inline in `from_config` before the creation of a
    // `System`.
    fn init_self(&mut self) {
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
}

pub struct Machine {
    _name: MachineName,
    provider: MachineProvider,
}

impl Machine {
    pub async fn build(&mut self) -> Result<()> {
        self.provider.build().await
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

    #[test]
    fn get_system_from_config_path() -> Result<()> {
        System::try_from_config_path(
            "../test_resources/systems/single-container-machine/conductor.toml",
        )?;

        Ok(())
    }
}
