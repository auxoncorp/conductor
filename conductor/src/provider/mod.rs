use crate::types::MachineName;
use std::path::PathBuf;

pub mod container;
pub mod gazebo;
pub mod qemu;
pub mod renode;

/// When we generate resources for the guest, they're placed in this root directory on the guest.
/// Providers are allowed to have their own further conventions within.
pub const GUEST_RESOURCES_PATH: &str = "/conductor_resources";

pub fn guest_machine_resource_path(m: &MachineName) -> PathBuf {
    PathBuf::from(GUEST_RESOURCES_PATH).join(m.as_str())
}
