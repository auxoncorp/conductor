use anyhow::Result;
use std::borrow::Cow;
use std::path::Path;

use crate::opts::{self, Build, Check};
use conductor::*;

pub async fn handle(s: opts::System) -> Result<()> {
    match s {
        opts::System::Check(Check { common }) => {
            let config_path: Cow<Path> = if let Some(config_path) = &common.config {
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
        opts::System::Build(Build { common }) => {
            let mut system = common.resolve_system()?;
            system.build().await?;
            println!("system built");
        }
        _ => todo!("system"),
    }

    Ok(())
}
