use crate::{Config, WorldOrMachineComponent};
use anyhow::Result;
use std::path::Path;

pub struct System {
    config: Config,
}

impl System {
    pub fn from_config(config: Config) -> Self {
        System { config }
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
