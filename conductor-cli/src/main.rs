mod opts;

use opts::{Command, System};
use std::error::Error;

use conductor_config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = opts::parse_args();

    match &args.command {
        Command::System(c) => match c {
            System::Check(check) => {
                println!("Checking configuration file '{}'", check.config.display());
                let cfg = Config::read(&check.config)?;
                println!("{cfg:#?}");
                Ok(())
            }
            _ => todo!("system"),
        },
        Command::Machine(_) => todo!("machine"),
    }
}
