use crate::config::BaseMachine;
use crate::types::ProviderKind;
use conductor_config::RenodeMachineProvider;
use derive_more::Display;
use std::path::PathBuf;

pub use resc::RenodeScriptGen;

mod resc;

pub const RESC_FILE_NAME: &str = "renode_script.resc";
pub const COMMAND: &str = "renode";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Renode", "self.base.name")]
pub struct RenodeMachine {
    pub base: BaseMachine,
    pub provider: RenodeMachineProvider,
}

impl RenodeMachine {
    // TODO - solidify a convention for this
    pub fn guest_resc_path(&self) -> PathBuf {
        PathBuf::from(format!("/{RESC_FILE_NAME}"))
    }
}
