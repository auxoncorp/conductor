use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RenodeMachineProvider {
    #[serde(flatten)]
    pub cli: RenodeCliConfig,

    #[serde(flatten)]
    pub resc: RenodeScriptConfig,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RenodeCliConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plain: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_xwt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_monitor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_log: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_analyzers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_temporary_files: Option<bool>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RenodeScriptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub platform_descriptions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_macro: Option<String>,
}
