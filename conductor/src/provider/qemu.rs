use crate::config::BaseMachine;
use crate::types::ProviderKind;
use conductor_config::QemuMachineProvider;
use derive_more::Display;

// TODO - determine this based on the bin field or explicit
// if bin is ELF, see what kind it is
pub const COMMAND: &str = "qemu-system-arm";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Qemu", "self.base.name")]
pub struct QemuMachine {
    pub base: BaseMachine,
    pub provider: QemuMachineProvider,
}
