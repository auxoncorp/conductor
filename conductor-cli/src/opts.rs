use clap::Parser;
use std::path::PathBuf;

pub fn parse_args() -> Args {
    Args::parse()
}

/// `conductor` - development and testing environment management for cyber-physical systems
///
/// Conductor is a [...]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about, disable_help_subcommand(true))]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug)]
pub enum Command {
    #[command(subcommand)]
    System(System),
    #[command(subcommand)]
    Machine(Machine),
}

#[derive(Parser, Debug)]
pub enum System {
    Check(Check),
    Create(Create),
    Start(Start),
    Stop(Stop),
    #[command(subcommand)]
    Export(Export),
}

/// Check a system
#[derive(Parser, Debug)]
pub struct Check {
    #[command(flatten)]
    pub common: CommonSystemOptions,
}

/// Create a new system
#[derive(Parser, Debug)]
pub struct Create {}

/// Bring up a system
#[derive(Parser, Debug)]
pub struct Start {}

/// Tear down a system
#[derive(Parser, Debug)]
pub struct Stop {}

/// Export a system
#[derive(Parser, Debug)]
pub enum Export {
    /// Export the system as a dot graph to stdout
    Graph {
        #[command(flatten)]
        common: CommonSystemOptions,

        /// Include color attributes for the nodes based on
        /// which container they belong to
        /// and the edges based on which connection they refer to.
        #[arg(long)]
        color: bool,

        /// Include direction attributes for the connections that
        /// are asymmetrical.
        #[arg(long)]
        directed: bool,
    },
}

#[derive(Parser, Debug)]
pub enum Machine {
    List(List),
    Inspect(Inspect),
    Dump(Dump),
}

/// List machines
#[derive(Parser, Debug)]
pub struct List {}

/// Inspect a machine
#[derive(Parser, Debug)]
pub struct Inspect {}

/// Dump a machine configuration
#[derive(Parser, Debug)]
pub struct Dump {}

#[derive(Parser, Debug)]
pub struct CommonSystemOptions {
    /// Path to config file.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

impl CommonSystemOptions {
    pub(crate) fn resolve_system(&self) -> anyhow::Result<conductor::System> {
        let sys = self
            .config
            .as_ref()
            .map(conductor::System::try_from_config_path)
            .unwrap_or_else(conductor::System::try_from_working_directory)?;
        Ok(sys)
    }
}
