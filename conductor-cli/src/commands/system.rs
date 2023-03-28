use anyhow::Result;
use std::borrow::Cow;
use std::path::Path;

use crate::opts;
use conductor::*;

// TODO
use conductor::component::WorldOrMachineComponent;
use conductor::component_graph::ComponentGraph;

pub fn handle(s: &opts::System) -> Result<()> {
    // TODO
    //let _system = System::try_from_working_directory()?;

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

            let components: Vec<WorldOrMachineComponent> = cfg
                .worlds
                .into_iter()
                .map(WorldOrMachineComponent::from)
                .chain(cfg.machines.into_iter().map(WorldOrMachineComponent::from))
                .collect();

            // TODO - add subcmd for 'system export topo or graph or w/e'
            // dot -Tx11 -Kcirco /tmp/system.dot
            // circo layout is usually better
            let graph = ComponentGraph::new(components, cfg.connections);
            let mut f = std::fs::File::create("/tmp/system.dot").unwrap();
            graph.write_dot(&mut f).unwrap();

            Ok(())
        }
        _ => todo!("system"),
    }
}
