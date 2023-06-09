use crate::{config::BaseWorld, provider::guest_component_resource_path, types::ProviderKind};
use conductor_config::GazeboWorldProvider;
use derive_more::Display;
use std::path::PathBuf;

const COMMAND: &str = "gz";

const DEFAULT_BASE_IMAGE: &str = "ghcr.io/auxoncorp/conductor-gazebo";

// See https://gazebosim.org/api/transport/12.0/envvars.html
// and https://gazebosim.org/api/gazebo/7/resources.html
// for other vars (previously prefixed with IGN, now GZ)
//
// Here's the Migration notes too with a list:
// https://github.com/gazebosim/gz-sim/blob/0e950b99f2dd5ee3694120195ae040e4aa82ece1/Migration.md#gazebo-sim-6x-to-70
const PARTITION_ENV_VAR: &str = "GZ_PARTITION";
const SYS_PLUGINS_ENV_VAR: &str = "GZ_SIM_SYSTEM_PLUGIN_PATH";
const RES_PATH_ENV_VAR: &str = "GZ_SIM_RESOURCE_PATH";

// Guest-relative files/dirs
const SYS_PLUGIN_DIR: &str = "system_plugins";
const RES_DIR: &str = "resources";
const GUI_CONFIG_FILE_NAME: &str = "gui.config";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Gazebo", "self.base.name")]
pub struct GazeboWorld {
    pub base: BaseWorld,
    pub provider: GazeboWorldProvider,
}

impl GazeboWorld {
    pub(crate) fn base_image(&self) -> String {
        DEFAULT_BASE_IMAGE.to_owned()
    }

    pub(crate) fn container_command(&self) -> String {
        COMMAND.to_owned()
    }

    pub(crate) fn container_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        args.push("sim".to_owned());
        if let Some(cfg) = self.guest_gui_config_path() {
            args.push("--gui-config".to_owned());
            args.push(cfg.to_string_lossy().to_string());
        }
        if self.provider.auto_start.unwrap_or(true) {
            args.push("-r".to_owned());
        }
        if self.provider.headless.unwrap_or(true) {
            args.push("--headless-rendering".to_owned());
            args.push("-s".to_owned());
        }
        if let Some(verbose) = self.provider.verbose {
            args.push("--verbose".to_owned());
            if verbose > 0 {
                args.push(verbose.to_string());
            }
        }
        args
    }

    pub(crate) fn guest_world(&self) -> PathBuf {
        // TODO - unwrap ok, already checked by config
        let sdf_file_name = self.provider.world_path.file_name().unwrap();
        guest_component_resource_path(&self.base.name).join(sdf_file_name)
    }

    pub(crate) fn guest_system_plugin_path(&self) -> Option<PathBuf> {
        if self.provider.plugin_path.is_some() {
            Some(guest_component_resource_path(&self.base.name).join(SYS_PLUGIN_DIR))
        } else {
            None
        }
    }

    pub(crate) fn guest_resource_path(&self) -> Option<PathBuf> {
        if self.provider.resource_path.is_some() {
            Some(guest_component_resource_path(&self.base.name).join(RES_DIR))
        } else {
            None
        }
    }

    pub(crate) fn guest_gui_config_path(&self) -> Option<PathBuf> {
        if self.provider.gui_config_path.is_some() {
            Some(
                guest_component_resource_path(&self.base.name)
                    .join(RES_DIR)
                    .join(GUI_CONFIG_FILE_NAME),
            )
        } else {
            None
        }
    }

    // NOTE: default partition is the world's name if not provided since it's
    // already checked for uniqueness within the system
    pub(crate) fn partition_env_kv(&self) -> (&str, String) {
        (
            PARTITION_ENV_VAR,
            self.provider
                .partition
                .clone()
                .unwrap_or_else(|| self.base.name.as_str().to_owned()),
        )
    }

    pub(crate) fn system_plugin_env_kv(&self) -> Option<(&str, String)> {
        self.guest_system_plugin_path()
            .map(|p| (SYS_PLUGINS_ENV_VAR, p.display().to_string()))
    }

    pub(crate) fn resource_env_kv(&self) -> Option<(&str, String)> {
        self.guest_resource_path()
            .map(|p| (RES_PATH_ENV_VAR, p.display().to_string()))
    }
}
