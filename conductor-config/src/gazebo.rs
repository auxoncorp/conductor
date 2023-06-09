use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GazeboWorldProvider {
    pub world_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<PathBuf>,
    // TODO - system_plugin vs gui_plugin paths, currently just system until we do GUI support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gui_config_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<usize>,
}
