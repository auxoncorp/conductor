use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ContainerMachineProvider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub containerfile: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}
