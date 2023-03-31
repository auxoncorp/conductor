use crate::{
    config::BaseMachine,
    provider::{guest_machine_resource_path, GUEST_RESOURCES_PATH},
    types::ProviderKind,
};
use conductor_config::RenodeMachineProvider;
use derive_more::Display;
use std::path::PathBuf;

pub use resc::RenodeScriptGen;

mod resc;

const RESC_FILE_NAME: &str = "renode_script.resc";
const COMMAND: &str = "renode";

pub(crate) fn guest_resc_path() -> PathBuf {
    // Starts at the res root, not prefixed with a machine since
    // this provider support multi-machines per single resc file
    PathBuf::from(GUEST_RESOURCES_PATH).join(RESC_FILE_NAME)
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Renode", "self.base.name")]
pub struct RenodeMachine {
    /// If this machine shares a bin with another renode machine
    /// on the same container, this will be true.
    /// Means the bin asset will be available in the resource root
    /// rather than nested under the machine.
    pub guest_bin_shared: bool,
    pub base: BaseMachine,
    pub provider: RenodeMachineProvider,
}

impl RenodeMachine {
    pub(crate) fn guest_bin(&self) -> PathBuf {
        // TODO - unwrap ok, already checked by config
        let bin_file_name = self.base.bin.file_name().unwrap();
        let base = if self.guest_bin_shared {
            PathBuf::from(GUEST_RESOURCES_PATH)
        } else {
            guest_machine_resource_path(&self.base.name)
        };
        base.join(bin_file_name)
    }

    pub(crate) fn container_command(&self) -> String {
        COMMAND.to_owned()
    }

    pub(crate) fn container_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        if self.provider.cli.plain.unwrap_or(false) {
            args.push("--plain".to_owned());
        }
        if let Some(p) = self.provider.cli.port {
            args.push("--port".to_owned());
            args.push(p.to_string());
        }
        if self.provider.cli.disable_xwt.unwrap_or(false) {
            args.push("--disable-xwt".to_owned());
        }
        if self.provider.cli.hide_monitor.unwrap_or(false) {
            args.push("--hide-monitor".to_owned());
        }
        if self.provider.cli.hide_log.unwrap_or(false) {
            args.push("--hide-log".to_owned());
        }
        if self.provider.cli.hide_analyzers.unwrap_or(false) {
            args.push("--hide-analyzers".to_owned());
        }
        if self.provider.cli.console.unwrap_or(false) {
            args.push("--console".to_owned());
        }
        if self.provider.cli.keep_temporary_files.unwrap_or(false) {
            args.push("--keep-temporary-files".to_owned());
        }
        args
    }
}
