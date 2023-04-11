use crate::{config::BaseMachine, provider::guest_component_resource_path, types::ProviderKind};
use conductor_config::QemuMachineProvider;
use derive_more::Display;
use std::path::PathBuf;

// TODO - determine this based on the bin field or explicit
// if bin is ELF, see what kind it is
const COMMAND: &str = "qemu-system-arm";

// TODO - change this
// build it from the root for now:
//   docker build -f images/qemu/Containerfile -t 'conductor_qemu:latest' images/qemu/
const DEFAULT_BASE_IMAGE: &str = "conductor_qemu:latest";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Qemu", "self.base.name")]
pub struct QemuMachine {
    pub base: BaseMachine,
    pub provider: QemuMachineProvider,
}

impl QemuMachine {
    pub(crate) fn base_image(&self) -> String {
        DEFAULT_BASE_IMAGE.to_owned()
    }

    // TODO - determine this based on the bin field or explicit
    // if bin is ELF, see what kind it is
    pub(crate) fn container_command(&self) -> String {
        COMMAND.to_owned()
    }

    pub(crate) fn container_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        // TODO
        // args that are local to qemu machine config
        // IO and shared stuff gets added later on
        //
        // should be enforced some for several of these
        if let Some(m) = &self.provider.machine {
            args.push("-machine".to_owned());
            args.push(m.to_owned());
        }
        if let Some(c) = &self.provider.cpu {
            args.push("-cpu".to_owned());
            args.push(c.to_owned());
        }
        if let Some(m) = &self.provider.memory {
            args.push("-m".to_owned());
            args.push(m.to_owned());
        }
        if self.provider.no_graphic.unwrap_or(false) {
            args.push("-nographic".to_owned());
        }
        args
    }

    pub(crate) fn guest_bin(&self) -> PathBuf {
        // TODO - unwrap ok, already checked by config
        let bin_file_name = self.base.bin.file_name().unwrap();
        guest_component_resource_path(&self.base.name).join(bin_file_name)
    }
}
