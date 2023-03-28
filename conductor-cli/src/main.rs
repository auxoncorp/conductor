mod commands;
mod opts;

use anyhow::Result;
use opts::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let args = opts::parse_args();

    tracing_subscriber::fmt::init();

    match args.command {
        Command::System(c) => commands::system::handle(c),
        Command::Machine(_) => todo!("machine"),
    }
}
