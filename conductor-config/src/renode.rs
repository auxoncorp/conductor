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
    pub plain: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub disable_xwt: bool,
    pub hide_monitor: bool,
    pub hide_log: bool,
    pub hide_analyzers: bool,
    pub console: bool,
    pub keep_temporary_files: bool,
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
