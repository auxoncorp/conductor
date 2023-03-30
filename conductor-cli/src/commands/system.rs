use anyhow::Result;
use std::borrow::Cow;
use std::{fs, io::Write, path::Path};

use crate::opts::{self, Build, Check, Start};
use conductor::provider::renode::{RenodeMachine, RenodeScriptGen, RESC_FILE_NAME};
use conductor::*;
use conductor::{config::Connection, types::ProviderKind};

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
            let graph = system.graph()?;
            println!("{:#?}", graph.components_by_container());
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
            opts::Export::ProviderArtifacts {
                common,
                output_path,
            } => {
                // TODO
                // this is just a debug tool atm
                // produces the following output directory layout and artifacts:
                // <out-dir>/
                //   <container_$idx>/
                //     <resources>/

                let root_dir = output_path;
                let system = common.resolve_system()?;
                let graph = system.graph()?;
                let mut renode_machines: Vec<RenodeMachine> = Vec::new();
                for (container_idx, container) in graph.components_by_container().iter().enumerate()
                {
                    let container_dir = root_dir.join(format!("container_{container_idx}"));
                    fs::create_dir_all(&container_dir)?;

                    let connections_for_this_container = container
                        .connections
                        .iter()
                        .map(|c| graph.connection(c).map(|n| n.clone()))
                        .collect::<std::result::Result<Vec<Connection>, _>>()?;

                    renode_machines.clear();
                    for component_name in container.components.iter() {
                        let component = graph.component(component_name)?;

                        // TODO sanity check entrypoint args for conlicts here or config
                        // layer
                        // for multi-component-per-container cases (renode)

                        let cmd = component.container_entrypoint_command();
                        let mut args = component.container_entrypoint_args();
                        let entrypoint_script_path = container_dir.join("entrypoint.sh");

                        // TODO ingore the lint for now, this is changing
                        #[allow(clippy::single_match)]
                        match component.provider() {
                            ProviderKind::Renode => {
                                let m = component.to_renode_machine().unwrap();

                                if renode_machines.is_empty() {
                                    args.push(m.guest_resc_path().display().to_string());

                                    let mut entrypoint_script_file =
                                        fs::File::create(entrypoint_script_path)?;
                                    writeln!(&mut entrypoint_script_file, "#!/usr/bin/env bash")?;
                                    writeln!(
                                        &mut entrypoint_script_file,
                                        "{cmd} {}",
                                        args.join(" ")
                                    )?;
                                }

                                renode_machines.push(m);
                            }
                            _ => {
                                let mut entrypoint_script_file =
                                    fs::File::create(entrypoint_script_path)?;
                                writeln!(&mut entrypoint_script_file, "#!/usr/bin/env bash")?;
                                writeln!(&mut entrypoint_script_file, "{cmd} {}", args.join(" "))?;
                            }
                        }
                    }

                    // Renode machines can be more than one to a container, so defered to here
                    if !renode_machines.is_empty() {
                        let resc_path = container_dir.join(RESC_FILE_NAME);
                        let mut resc_file = fs::File::create(resc_path)?;
                        RenodeScriptGen::new(&mut resc_file)
                            .generate(&renode_machines, &connections_for_this_container)?;
                    }
                }
            }
        },
        opts::System::Build(Build { common }) => {
            let mut system = common.resolve_system()?;
            system.build().await?;
            println!("system built");
        }
        opts::System::Start(Start { common }) => {
            let mut system = common.resolve_system()?;
            system.start().await?;
            println!("system started");
        }
        _ => todo!("system"),
    }

    Ok(())
}
