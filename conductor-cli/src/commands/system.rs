use anyhow::Result;
use std::borrow::Cow;
use std::path::Path;

use crate::opts;
use conductor::*;

// TODO
use conductor::component_graph::ComponentGraph;
use conductor::config::MachineProvider;
use conductor::provider::renode::RenodeMachine;

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

            let renode_machines: Vec<RenodeMachine> = cfg
                .machines
                .into_iter()
                .map(|m| match m.provider {
                    MachineProvider::Renode(p) => RenodeMachine {
                        base: m.base,
                        provider: p,
                    },
                    _ => todo!(),
                })
                .collect();

            // TODO - add subcmd for 'system export topo or graph or w/e'
            let graph = ComponentGraph::new(renode_machines, cfg.connections);
            let mut f = std::fs::File::create("/tmp/system.dot").unwrap();
            graph.write_dot(&mut f).unwrap();

            Ok(())
        }
        _ => todo!("system"),
    }
}
