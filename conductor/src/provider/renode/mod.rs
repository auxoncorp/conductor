use crate::component::Component;
use crate::config::{BaseMachine, MachineConnector};
use crate::types::{
    ComponentName, ConnectionKind, EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths,
    ProviderKind,
};
use conductor_config::RenodeMachineProvider;
use derive_more::Display;

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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "self.provider()", "self.base.name")]
pub struct RenodeMachine {
    pub base: BaseMachine,
    pub provider: RenodeMachineProvider,
}

impl Component for RenodeMachine {
    fn name(&self) -> ComponentName {
        self.base.name.clone().into()
    }

    fn provider(&self) -> ProviderKind {
        ProviderKind::Renode
    }

    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs {
        &self.base.environment_variables
    }

    fn assets(&self) -> &HostToGuestAssetPaths {
        &self.base.assets
    }

    fn connectors(&self) -> &[MachineConnector] {
        &self.base.connectors
    }
}
