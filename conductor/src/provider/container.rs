// TODO - this is currently just a stub

use crate::config::BaseMachine;
use crate::types::ProviderKind;
use conductor_config::ContainerMachineProvider;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Container", "self.base.name")]
pub struct ContainerMachine {
    pub base: BaseMachine,
    pub provider: ContainerMachineProvider,
}
