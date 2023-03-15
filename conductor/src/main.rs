use conductor_cli::Command;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = conductor_cli::parse_args();

    match &args.command {
        Command::System(_) => todo!("system"),
        Command::Machine(_) => todo!("machine"),
    }
}
