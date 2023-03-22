use crate::Config;
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
        println!("{}", path.as_ref().display());
        let config = Config::read(path)?;

        Ok(System { config })
    }

    pub fn try_from_working_directory() -> Result<Self> {
        let config_path = conductor_config::find_config_file()?;
        let config = Config::read(config_path)?;

        Ok(System { config })
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_system_from_config_path() -> Result<()> {
        System::try_from_config_path(
            "../test_resources/systems/single-docker-machine/conductor.toml",
        )?;

        Ok(())
    }
}
