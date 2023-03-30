use crate::config::{Machine, MachineConnector, World, WorldConnector};
use crate::types::{
    ComponentName, ConnectionName, EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths,
    ProviderKind,
};
use derive_more::{Display, From};

pub trait Component {
    fn name(&self) -> ComponentName;
    fn provider(&self) -> ProviderKind;
    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs;
    fn assets(&self) -> &HostToGuestAssetPaths;
    fn connectors(&self) -> Vec<ComponentConnector>;
    fn container_entrypoint_command(&self) -> String;
    fn container_entrypoint_args(&self) -> Vec<String>;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From, Display)]
pub enum WorldOrMachineComponent {
    World(World),
    Machine(Machine),
}

impl Component for WorldOrMachineComponent {
    fn name(&self) -> ComponentName {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::name(c),
            Machine(c) => Component::name(c),
        }
    }
    fn provider(&self) -> ProviderKind {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::provider(c),
            Machine(c) => Component::provider(c),
        }
    }
    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::environment_variables(c),
            Machine(c) => Component::environment_variables(c),
        }
    }
    fn assets(&self) -> &HostToGuestAssetPaths {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::assets(c),
            Machine(c) => Component::assets(c),
        }
    }
    fn connectors(&self) -> Vec<ComponentConnector> {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::connectors(c),
            Machine(c) => Component::connectors(c),
        }
    }
    fn container_entrypoint_command(&self) -> String {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::container_entrypoint_command(c),
            Machine(c) => Component::container_entrypoint_command(c),
        }
    }
    fn container_entrypoint_args(&self) -> Vec<String> {
        use WorldOrMachineComponent::*;
        match self {
            World(c) => Component::container_entrypoint_args(c),
            Machine(c) => Component::container_entrypoint_args(c),
        }
    }
}

impl WorldOrMachineComponent {
    // TODO - not sure this pattern will stick around
    pub fn to_renode_machine(&self) -> Option<crate::provider::renode::RenodeMachine> {
        use crate::config::MachineProvider::*;
        use WorldOrMachineComponent::*;
        match self {
            World(_) => None,
            Machine(m) => match &m.provider {
                Renode(r) => Some(crate::provider::renode::RenodeMachine {
                    base: m.base.clone(),
                    provider: r.clone(),
                }),
                _ => None,
            },
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
pub enum ComponentConnector {
    World(WorldConnector),
    Machine(MachineConnector),
}

impl ComponentConnector {
    pub fn name(&self) -> &ConnectionName {
        use ComponentConnector::*;
        match self {
            World(c) => &c.name,
            Machine(c) => &c.name,
        }
    }

    pub fn is_asymmetrical_initiator(&self) -> Option<bool> {
        use ComponentConnector::*;
        match self {
            World(_c) => None,
            Machine(c) => c.properties.is_asymmetrical_initiator(),
        }
    }
}
