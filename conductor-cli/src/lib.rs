use clap::Parser;

pub fn parse_args() -> Args {
    Args::parse()
}

/// `conductor` - development and testing environment management for cyber-physical systems
///
/// Conductor is a [...]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about, disable_help_subcommand(true))]
pub struct Args {
    /// Make logging more verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Make logging less verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    pub quiet: u8,

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
    Create(Create),
    Start(Start),
    Stop(Stop),
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
