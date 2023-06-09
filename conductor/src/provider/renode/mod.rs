use crate::{
    config::BaseMachine,
    provider::{guest_component_resource_path, GUEST_RESOURCES_PATH},
    types::{BridgeName, ConnectionName, ProviderKind, TapDevice},
};
use conductor_config::RenodeMachineProvider;
use derive_more::{AsRef, Deref, Display, From};
use std::{collections::BTreeMap, path::PathBuf};

pub use platform_description::PlatformDescription;
pub use resc::RenodeScriptGen;

mod platform_description;
mod resc;

const COMMAND: &str = "renode";
const RESC_FILE_NAME: &str = "renode_script.resc";
const NET_SETUP_FILE_NAME: &str = "net_setup.sh";
const NET_TEARDOWN_FILE_NAME: &str = "net_teardown.sh";

const DEFAULT_BASE_IMAGE: &str = "ghcr.io/auxoncorp/conductor-renode";

// NOTE: this will be expanded to deal with URIs and other types later
//pub enum Executable { Elf(PathBuf), ... }
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, AsRef, Deref, From)]
pub struct Executable(pub PathBuf);

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
    /// These platform descriptions are constructed from the
    /// config-level string descriptions found in provider.resc.platform_descriptions
    pub platform_descriptions: Vec<PlatformDescription>,
    pub executable: Executable,
    pub tap_devices: BTreeMap<ConnectionName, TapDevice>,
}

impl RenodeMachine {
    pub(crate) fn base_image(&self) -> String {
        DEFAULT_BASE_IMAGE.to_owned()
    }

    pub(crate) fn guest_bin(&self) -> PathBuf {
        // TODO - unwrap ok, already checked by config
        let bin_file_name = self.executable.file_name().unwrap();
        let base = if self.guest_bin_shared {
            PathBuf::from(GUEST_RESOURCES_PATH)
        } else {
            guest_component_resource_path(&self.base.name)
        };
        base.join(bin_file_name)
    }

    pub(crate) fn container_command(&self) -> String {
        COMMAND.to_owned()
    }

    pub(crate) fn container_args(&self) -> Vec<String> {
        const DEFAULT_HEADLESS_CONSOLE: bool = true;
        let mut args: Vec<String> = Vec::new();
        if self.provider.cli.plain.unwrap_or(false) {
            args.push("--plain".to_owned());
        }
        if let Some(p) = self.provider.cli.port {
            args.push("--port".to_owned());
            args.push(p.to_string());
        }
        if self
            .provider
            .cli
            .disable_xwt
            .unwrap_or(DEFAULT_HEADLESS_CONSOLE)
        {
            args.push("--disable-xwt".to_owned());
        }
        if self.provider.cli.hide_monitor.unwrap_or(false) {
            args.push("--hide-monitor".to_owned());
        }
        if self
            .provider
            .cli
            .hide_log
            .unwrap_or(DEFAULT_HEADLESS_CONSOLE)
        {
            args.push("--hide-log".to_owned());
        }
        if self.provider.cli.hide_analyzers.unwrap_or(false) {
            args.push("--hide-analyzers".to_owned());
        }
        if self
            .provider
            .cli
            .console
            .unwrap_or(DEFAULT_HEADLESS_CONSOLE)
        {
            args.push("--console".to_owned());
        }
        if self.provider.cli.keep_temporary_files.unwrap_or(false) {
            args.push("--keep-temporary-files".to_owned());
        }
        args
    }
}

// NOTE:
// * on the host, this requires CAP_NET_ADMIN (docker --cap-add=NET_ADMIN)
// * on the guest, requires things from the iproute2 and bridge-utils packages
pub(crate) fn external_network_setup_script_content(
    taps_to_bridges: &BTreeMap<TapDevice, BridgeName>,
) -> String {
    // 10, 200 is device code for TAP/TUN
    // https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/Documentation/networking/tuntap.rst
    const PRE: &str = indoc::indoc! {r#"
        #!/usr/bin/env bash
        set -euo pipefail
        mkdir -p /dev/net
        mknod /dev/net/tun c 10 200
    "#};
    let mut script = String::new();
    script.push_str(PRE);
    for (tap, bridge) in taps_to_bridges.iter() {
        script.push_str(&format!(
            indoc::indoc! {r#"
                ip tuntap add dev {tap} mode tap
                ip link set dev {tap} up
                brctl addif {bridge} {tap}
            "#},
            bridge = bridge,
            tap = tap
        ));
    }
    script.push_str("exit 0\n");
    script
}

pub(crate) fn external_network_teardown_script_content(
    taps_to_bridges: &BTreeMap<TapDevice, BridgeName>,
) -> String {
    const PRE: &str = indoc::indoc! {r#"
        #!/usr/bin/env bash
        # We're in the container, treat these as idempotent
        #set -euo pipefail
    "#};
    let mut script = String::new();
    script.push_str(PRE);
    for (tap, bridge) in taps_to_bridges.iter() {
        script.push_str(&format!(
            indoc::indoc! {r#"
                brctl delif {bridge} {tap}
                ip link set dev {tap} down
                ip tuntap del {tap} mode tap
            "#},
            bridge = bridge,
            tap = tap
        ));
    }
    script.push_str("exit 0\n");
    script
}

pub(crate) fn guest_resc_path() -> PathBuf {
    // Starts at the res root, not prefixed with a machine since
    // this provider support multi-machines per single resc file
    PathBuf::from(GUEST_RESOURCES_PATH).join(RESC_FILE_NAME)
}

pub(crate) fn guest_external_network_setup_script_path() -> PathBuf {
    PathBuf::from(GUEST_RESOURCES_PATH).join(NET_SETUP_FILE_NAME)
}

pub(crate) fn guest_external_network_teardown_script_path() -> PathBuf {
    PathBuf::from(GUEST_RESOURCES_PATH).join(NET_TEARDOWN_FILE_NAME)
}
