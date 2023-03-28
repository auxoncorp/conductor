use anyhow::Result;
use std::borrow::Cow;
use std::path::Path;

use crate::opts;
use conductor::*;

pub fn handle(s: opts::System) -> Result<()> {
    match s {
        opts::System::Check(check) => {
            let config_path: Cow<Path> = if let Some(config_path) = &check.common.config {
                config_path.into()
            } else {
                conductor_config::find_config_file()?.into()
            };
            println!("Checking configuration file '{}'", config_path.display());

            let system = System::try_from_config_path(&config_path)?;
            // TODO - rm this print at some point, probably show some summary details
            println!("{:#?}", system.config());
        }
        opts::System::Export(export) => match export {
            opts::Export::Graph {
                common,
                color,
                directed,
            } => {
                let system = common.resolve_system()?;
                let components = system.components();
                let mut stdout = std::io::stdout().lock();
                let graph = ComponentGraph::new(components, system.config().connections.clone())?;
                graph.write_dot(color, directed, &mut stdout)?;
            }
        },
        _ => todo!("system"),
    }

    Ok(())
}
