use crate::opts::{Attach, Dump, Inspect, List, Machine, Shell, Stats};
use crate::stats::ContainerAndStats;
use anyhow::{bail, Result};
use conductor::containers::{Container, LogOutput, StartExecResults};
use futures_util::StreamExt;
use std::io::{self, Write};
use tabwriter::TabWriter;
use tokio::io::AsyncWriteExt;

pub async fn handle(s: Machine) -> Result<()> {
    match s {
        Machine::List(List { .. }) => {
            todo!("machine list");
        }
        Machine::Inspect(Inspect { .. }) => {
            todo!("machine inspect");
        }
        Machine::Attach(Attach {
            system,
            machine_name,
        }) => {
            let system = system.resolve_system().await?;

            let container_runtime_name =
                system.container_runtime_name_for_machine_named(&machine_name)?;

            for container in system.containers() {
                if container
                    .name()
                    .map(|n| n == container_runtime_name.as_str())
                    == Some(true)
                {
                    attach_to_container(container).await?;
                    break;
                }
            }
        }
        Machine::Stats(Stats {
            system,
            machine_name,
        }) => {
            let system = system.resolve_system().await?;

            let container_runtime_name =
                system.container_runtime_name_for_machine_named(&machine_name)?;

            for container in system.containers() {
                if container
                    .name()
                    .map(|n| n == container_runtime_name.as_str())
                    == Some(true)
                {
                    let stats =
                        ContainerAndStats::new(machine_name.into(), container.stats().await?);

                    let mut tw = TabWriter::new(io::stdout());
                    writeln!(tw, "{}", ContainerAndStats::TABWRITER_HEADER)?;
                    stats.tabwriter_writeln(&mut tw)?;
                    tw.flush()?;

                    break;
                }
            }
        }
        Machine::Dump(Dump { .. }) => {
            todo!("machine dump");
        }
        Machine::Shell(Shell {
            system,
            machine_name,
        }) => {
            // TODO: find machine
            let system = system.resolve_system().await?;

            let container = system.get_container_by_component_name(&machine_name)?;

            // TODO: exec a shell in machine
            //
            // TODO: attach to newly exec'd thing
            shell_for_container(container).await?;
        }
    }

    Ok(())
}

async fn attach_to_container(container: &Container) -> Result<()> {
    let io = container.attach().await?;

    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();

    let mut input = io.input;
    let mut output = io.output;

    tokio::spawn(async move {
        tokio::io::copy(&mut stdin, &mut input)
            .await
            .expect("connect stdin");
    });

    while let Some(output_item) = output.next().await {
        let cmd_output = output_item?;

        match cmd_output {
            LogOutput::StdIn { message } => stdout.write_all(&message).await?,
            LogOutput::StdOut { message } => stdout.write_all(&message).await?,
            LogOutput::StdErr { message } => stderr.write_all(&message).await?,
            // Everything is coming out of this variant, why? What actaully is it?
            LogOutput::Console { message } => stdout.write_all(&message).await?,
        }
    }

    Ok(())
}

async fn shell_for_container(container: &Container) -> Result<()> {
    let io = container.shell().await?;

    let StartExecResults::Attached {
        mut input, mut output
    } = io else {
        bail!("docker didn't attach to exec'd shell")
    };

    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();

    tokio::spawn(async move {
        tokio::io::copy(&mut stdin, &mut input)
            .await
            .expect("connect stdin");
    });

    while let Some(output_item) = output.next().await {
        let cmd_output = output_item?;

        match cmd_output {
            LogOutput::StdIn { message } => stdout.write_all(&message).await?,
            LogOutput::StdOut { message } => stdout.write_all(&message).await?,
            LogOutput::StdErr { message } => stderr.write_all(&message).await?,
            // Everything is coming out of this variant, why? What actaully is it?
            LogOutput::Console { message } => stdout.write_all(&message).await?,
        }
    }

    Ok(())
}
