mod commands;
mod opts;
mod stats;

use anyhow::Result;
use opts::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let args = opts::parse_args();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .pretty()
        .init();

    match args.command {
        Command::System(c) => commands::system::handle(c).await,
        Command::Machine(m) => commands::machine::handle(m).await,
    }
}
