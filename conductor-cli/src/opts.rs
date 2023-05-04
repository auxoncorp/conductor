use clap::Parser;
use conductor::types::MachineName;
use std::{path::PathBuf, str::FromStr};

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
    Build(Build),
    Start(Start),
    Stop(Stop),
    Stats(SystemStats),
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
pub struct Build {
    #[command(flatten)]
    pub common: CommonSystemOptions,
}

/// Bring up a system
#[derive(Parser, Debug)]
pub struct Start {
    #[command(flatten)]
    pub common: CommonSystemOptions,
}

/// Tear down a system
#[derive(Parser, Debug)]
pub struct Stop {}

/// Show stats for each of the components in a system
#[derive(Parser, Debug)]
pub struct SystemStats {
    #[command(flatten)]
    pub common: CommonSystemOptions,
}

/// Export a system
#[derive(Parser, Debug)]
pub enum Export {
    /// Export the system as a dot graph to stdout
    Graph {
        #[command(flatten)]
        common: CommonSystemOptions,

        /// Graph format to use
        #[arg(short = 'f', long)]
        format: GraphFormat,

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

    /// Export the system's internal deployment plan
    DeploymentPlan {
        #[command(flatten)]
        common: CommonSystemOptions,

        /// Output directory
        #[arg(default_value = "deployment_plan")]
        output_path: PathBuf,
    },
}

#[derive(Parser, Debug)]
pub enum Machine {
    List(List),
    Inspect(Inspect),
    Attach(Attach),
    Stats(Stats),
    Dump(Dump),
}

/// List machines
#[derive(Parser, Debug)]
pub struct List {
    #[command(flatten)]
    pub system: CommonSystemOptions,
}

/// Inspect a machine
#[derive(Parser, Debug)]
pub struct Inspect {
    #[command(flatten)]
    pub system: CommonSystemOptions,
}

/// Attach to a running machine
#[derive(Parser, Debug)]
pub struct Attach {
    #[command(flatten)]
    pub system: CommonSystemOptions,

    pub machine_name: MachineName,
}

/// Print machine stats
#[derive(Parser, Debug)]
pub struct Stats {
    #[command(flatten)]
    pub system: CommonSystemOptions,

    pub machine_name: MachineName,
}

/// Dump a machine configuration
#[derive(Parser, Debug)]
pub struct Dump {
    #[command(flatten)]
    pub system: CommonSystemOptions,
}

#[derive(Parser, Debug)]
pub struct CommonSystemOptions {
    /// Path to config file.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

impl CommonSystemOptions {
    pub(crate) async fn resolve_system(&self) -> anyhow::Result<conductor::System> {
        if let Some(ref config) = self.config {
            conductor::System::try_from_config_path(config).await
        } else {
            conductor::System::try_from_working_directory().await
        }
    }
}

#[derive(Parser, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub enum GraphFormat {
    /// Graphviz dot format
    #[default]
    Dot,

    /// Mermaid format
    Mermaid,
}

impl FromStr for GraphFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dot" => Ok(GraphFormat::Dot),
            "mermaid" => Ok(GraphFormat::Mermaid),
            _ => Err(format!("'{s}' is not a valid GraphFormat kind")),
        }
    }
}
