use crate::config::MachineConnector;
use crate::types::{
    ComponentName, EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths, ProviderKind,
};

pub trait Component {
    fn name(&self) -> ComponentName;
    fn provider(&self) -> ProviderKind;
    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs;
    fn assets(&self) -> &HostToGuestAssetPaths;
    // TODO Connector with machine/etc variants once we tie together the world/sim
    // pieces
    fn connectors(&self) -> &[MachineConnector];
}
