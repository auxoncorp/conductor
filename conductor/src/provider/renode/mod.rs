use crate::config::BaseMachine;
use crate::types::ProviderKind;
use conductor_config::RenodeMachineProvider;
use derive_more::Display;

pub use resc::RenodeScriptGen;

mod resc;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Renode", "self.base.name")]
pub struct RenodeMachine {
    pub base: BaseMachine,
    pub provider: RenodeMachineProvider,
}
