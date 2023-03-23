use crate::{config::BaseMachine, types::ConnectionKind};
use conductor_config::RenodeMachineProvider;

pub use resc::RenodeScriptGen;

mod resc;

pub trait RenodeConnectionKindExt {
    fn is_restricted_to_single_renode_context(&self) -> bool;
}

impl RenodeConnectionKindExt for ConnectionKind {
    fn is_restricted_to_single_renode_context(&self) -> bool {
        use ConnectionKind::*;
        match self {
            Uart => false,
            Gpio => true,
            Network => false,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RenodeMachine {
    pub base: BaseMachine,
    pub provider: RenodeMachineProvider,
}
