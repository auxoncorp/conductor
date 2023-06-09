use anyhow::{anyhow, Result};
use std::borrow::Cow;
use std::io::{self, Write};
use std::{collections::BTreeMap, fs, path::Path};
use tabwriter::TabWriter;

use crate::{
    opts::{self, Build, Check, GraphFormat, Start, SystemStats, Watch},
    stats::ContainerAndStats,
    tui::watch::WatchApp,
};
use conductor::types::ContainerRuntimeName;
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

            let system = System::try_from_config_path(&config_path).await?;
            // TODO - rm this print at some point, probably show some summary details
            println!("{:#?}", system.config());
            let graph = system.graph()?;
            println!("{:#?}", graph.components_by_container());
        }
        opts::System::Export(export) => match export {
            opts::Export::Graph {
                common,
                format,
                color,
                directed,
            } => {
                let system = common.resolve_system().await?;
                let graph = system.graph()?;
                let mut stdout = std::io::stdout().lock();
                match format {
                    GraphFormat::Dot => graph.write_dot(color, directed, &mut stdout)?,
                    GraphFormat::Mermaid => graph.write_mermaid(color, directed, &mut stdout)?,
                }
            }
            opts::Export::DeploymentPlan {
                common,
                output_path,
            } => {
                let system = common.resolve_system().await?;
                let deployment = system.deployment()?;
                let root_dir = output_path.join(deployment.system_name.as_str());

                let mut container_idx = 0;
                for c in deployment.gazebo_containers.iter() {
                    gen_container_deployment_plan(&root_dir, container_idx, c)?;
                    container_idx += 1;
                }
                for c in deployment.renode_containers.iter() {
                    gen_container_deployment_plan(&root_dir, container_idx, c)?;
                    container_idx += 1;
                }
                for c in deployment.qemu_containers.iter() {
                    gen_container_deployment_plan(&root_dir, container_idx, c)?;
                    container_idx += 1;
                }
                for c in deployment.container_containers.iter() {
                    gen_container_deployment_plan(&root_dir, container_idx, c)?;
                    container_idx += 1;
                }
            }
        },
        opts::System::Build(Build { common }) => {
            let mut system = common.resolve_system().await?;
            system.build().await?;
            println!("system built");
        }
        opts::System::Start(Start { common }) => {
            let mut system = common.resolve_system().await?;
            system.start().await?;
            println!("system started");
        }
        opts::System::Stats(SystemStats { common }) => {
            let system = common.resolve_system().await?;
            let mut tw = TabWriter::new(io::stdout());
            writeln!(tw, "{}", ContainerAndStats::TABWRITER_HEADER)?;
            for container in system.containers() {
                let (_system_name, component_names) = container
                    .name()
                    .and_then(ContainerRuntimeName::extract_components)
                    .ok_or_else(|| {
                        anyhow!("Failed to resolve the system component runtime container names")
                    })?;

                let name = if component_names.len() == 1 {
                    component_names[0].to_string()
                } else {
                    format!(
                        "[{}]",
                        component_names
                            .iter()
                            .map(|c| c.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    )
                };

                let stats = ContainerAndStats::new(name, container.stats().await?);
                stats.tabwriter_writeln(&mut tw)?;
            }
            tw.flush()?;
        }
        opts::System::Watch(Watch { common }) => {
            let system = common.resolve_system().await?;

            WatchApp::new(system).run().await?;
        }
        _ => todo!("system"),
    }

    Ok(())
}

fn gen_container_deployment_plan<P: AsRef<Path>, C>(
    root_dir: P,
    container_idx: usize,
    c: &DeploymentContainer<C>,
) -> Result<()> {
    let container_dir = root_dir.as_ref().join(format!("container_{container_idx}"));
    fs::create_dir_all(&container_dir)?;

    for (guest_file_path, contents) in c.generated_guest_files.iter() {
        let file_name = guest_file_path.file_name().unwrap();
        let file_path = container_dir.join(file_name);
        fs::write(file_path, contents)?;
    }

    // Why json? idk
    let plan_path = container_dir.join("plan.json");
    let plan = serde_json::to_string_pretty(&serde_json::json!({
        "name" : *c.name,
        "uses_host_display" : c.uses_host_display,
        "environment_variables": *c.environment_variables,
        "assets": *c.assets,
        "command": c.command,
        "args": c.args,
        "networks": c.connections.iter().filter(|c| c.is_network()).map(|c| c.name().as_str()).collect::<Vec<&str>>(),
        "taps_to_bridges": c.taps_to_bridges.iter().map(|(t, b)| (t.as_str(), b.as_str())).collect::<BTreeMap<&str, &str>>(),
    }))?;
    fs::write(plan_path, plan)?;

    Ok(())
}
