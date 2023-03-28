use crate::config::BaseWorld;
use crate::types::ProviderKind;
use conductor_config::GazeboWorldProvider;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Gazebo", "self.base.name")]
pub struct GazeboWorld {
    pub base: BaseWorld,
    pub provider: GazeboWorldProvider,
}
