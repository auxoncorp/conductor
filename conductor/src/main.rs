use conductor_cli::Command;
use std::error::Error;

mod config;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = conductor_cli::parse_args();

    match &args.command {
        Command::System(c) => match c {
            conductor_cli::System::Check(check) => {
                println!("Checking configuration file '{}'", check.config.display());
                let cfg = config::Config::read(&check.config)?;
                println!("{cfg:#?}");
                Ok(())
            }
            _ => todo!("system"),
        },
        Command::Machine(_) => todo!("machine"),
    }
}
