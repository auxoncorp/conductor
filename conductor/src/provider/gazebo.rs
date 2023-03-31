use crate::config::BaseWorld;
use crate::types::ProviderKind;
use conductor_config::GazeboWorldProvider;
use derive_more::Display;

const COMMAND: &str = "gz";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}:{}", "ProviderKind::Gazebo", "self.base.name")]
pub struct GazeboWorld {
    pub base: BaseWorld,
    pub provider: GazeboWorldProvider,
}

impl GazeboWorld {
    pub(crate) fn container_command(&self) -> String {
        COMMAND.to_owned()
    }

    pub(crate) fn container_args(&self) -> Vec<String> {
        let mut args: Vec<String> = Vec::new();
        // TODO
        // will look like this
        // gz sim -r world.sdf
        args.push("sim".to_owned());
        if self.provider.headless.unwrap_or(false) {
            args.push("-r".to_owned());
        }
        // TODO path in cfg, not fixed
        args.push("world.sdf".to_owned());
        args
    }
}
