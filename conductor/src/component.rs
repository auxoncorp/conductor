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
}

impl<T: Component + ?Sized> Component for Box<T> {
    fn name(&self) -> ComponentName {
        T::name(self)
    }
    fn provider(&self) -> ProviderKind {
        T::provider(self)
    }
    fn environment_variables(&self) -> &EnvironmentVariableKeyValuePairs {
        T::environment_variables(self)
    }
    fn assets(&self) -> &HostToGuestAssetPaths {
        T::assets(self)
    }
    fn connectors(&self) -> Vec<ComponentConnector> {
        T::connectors(self)
    }
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
