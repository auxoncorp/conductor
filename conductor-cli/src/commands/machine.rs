use crate::opts::{Attach, Dump, Inspect, List, Machine};
use anyhow::Result;
use conductor::containers::LogOutput;
use futures_util::StreamExt;
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

                    break;
                }
            }
        }
        Machine::Dump(Dump { .. }) => {
            todo!("machine dump");
        }
    }

    Ok(())
}
