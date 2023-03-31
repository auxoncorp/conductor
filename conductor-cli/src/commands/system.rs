use anyhow::Result;
use std::borrow::Cow;
use std::{fs, path::Path};

use crate::opts::{self, Build, Check, Start};
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
            opts::Export::DeploymentPlan {
                common,
                output_path,
            } => {
                let system = common.resolve_system()?;
                let root_dir = output_path.join(system.config().global.name.as_str());
                let graph = system.graph()?;
                let deployment = Deployment::from_graph(&graph)?;

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
            let mut system = common.resolve_system()?;
            system.init_self(); // TODO - see TODO in system mod
            system.build().await?;
            println!("system built");
        }
        opts::System::Start(Start { common }) => {
            let mut system = common.resolve_system()?;
            system.init_self(); // TODO - see TODO in system mod
            system.start().await?;
            println!("system started");
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
        "environment_variables": *c.environment_variables,
        "assets": *c.assets,
        "command": c.command,
        "args": c.args,
    }))?;
    fs::write(plan_path, plan)?;

    Ok(())
}
