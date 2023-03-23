use anyhow::Result;
use std::borrow::Cow;
use std::path::Path;

use crate::opts;
use conductor::*;

pub fn handle(s: &opts::System) -> Result<()> {
    let _system = System::try_from_working_directory()?;

    match s {
        opts::System::Check(check) => {
            let config_path: Cow<Path> = if let Some(config_path) = &check.config {
                config_path.into()
            } else {
                conductor_config::find_config_file()?.into()
            };
            println!("Checking configuration file '{}'", config_path.display());
            let cfg = config::Config::read(&config_path)?;
            println!("{cfg:#?}");
            Ok(())
        }
        _ => todo!("system"),
    }
}
