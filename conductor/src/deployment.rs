use crate::{
    config::{Connection, MachineProvider, WorldProvider},
    provider::{
        container::ContainerMachine,
        gazebo::GazeboWorld,
        qemu::QemuMachine,
        renode::{guest_resc_path, RenodeMachine, RenodeScriptGen},
    },
    types::{EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths, ProviderKind},
    Component, ComponentGraph, WorldOrMachineComponent,
};
use anyhow::Result;
use std::{collections::BTreeMap, path::PathBuf, str};

// TODO Error type with contextual variants probably

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeploymentContainer<C> {
    pub environment_variables: EnvironmentVariableKeyValuePairs,
    pub assets: HostToGuestAssetPaths,
    pub generated_guest_files: BTreeMap<PathBuf, String>,
    pub command: String,
    pub args: Vec<String>,
    pub connections: Vec<Connection>,
    pub components: Vec<C>,
}

impl<C> Default for DeploymentContainer<C> {
    fn default() -> Self {
        DeploymentContainer {
            environment_variables: Default::default(),
            assets: Default::default(),
            generated_guest_files: Default::default(),
            command: Default::default(),
            args: Default::default(),
            connections: Vec::new(),
            components: Vec::new(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Deployment {
    pub gazebo_containers: Vec<DeploymentContainer<GazeboWorld>>,
    pub renode_containers: Vec<DeploymentContainer<RenodeMachine>>,
    pub qemu_containers: Vec<DeploymentContainer<QemuMachine>>,
    pub container_containers: Vec<DeploymentContainer<ContainerMachine>>,
}

impl Deployment {
    pub fn from_graph(graph: &ComponentGraph<WorldOrMachineComponent>) -> Result<Self> {
        let mut gazebo_containers = Vec::new();
        let mut renode_containers = Vec::new();
        let mut qemu_containers = Vec::new();
        let mut container_containers = Vec::new();

        for container in graph.components_by_container().iter() {
            // Renode provider can have multiple machines so its fields can be merged
            // as we iterator over each machine
            let mut renode_container: DeploymentContainer<RenodeMachine> = Default::default();

            let connections = container
                .connections
                .iter()
                .map(|c| graph.connection(c).map(|n| n.clone()))
                .collect::<std::result::Result<Vec<Connection>, _>>()?;

            // TODO
            // debug_assert all providers in container match
            let provider = container
                .components
                .iter()
                .next()
                .map(|c| graph.component(c).map(|cmp| cmp.provider()))
                .unwrap()?; // TODO

            if provider == ProviderKind::Renode {
                renode_container.connections = connections.clone();
            } else {
                debug_assert!(
                    container.components.len() == 1,
                    " The provider {provider} only supports one component per container"
                );
            }

            for component_name in container.components.iter() {
                let component = graph.component(component_name)?.clone();

                match component {
                    WorldOrMachineComponent::World(w) => match w.provider {
                        WorldProvider::Gazebo(p) => {
                            let gw = GazeboWorld {
                                base: w.base,
                                provider: p,
                            };

                            // Add world path to assets
                            let mut assets = gw.base.assets.clone();
                            assets.insert(gw.provider.world_path.clone(), gw.guest_world())?;

                            // Add world path to args
                            let mut args = gw.container_args();
                            args.push(gw.guest_world().display().to_string());

                            // Add gazebo path env vars
                            let mut environment_variables = gw.base.environment_variables.clone();

                            if let Some((k, v)) = gw.system_plugin_env_kv() {
                                environment_variables.insert(k.to_owned(), v)?;
                            }

                            if let Some((k, v)) = gw.resource_env_kv() {
                                environment_variables.insert(k.to_owned(), v)?;
                            }

                            // Same for assets
                            if let Some(host_p) = &gw.provider.plugin_path {
                                let guest_p = gw.guest_system_plugin_path().unwrap();
                                assets.insert(host_p.clone(), guest_p)?;
                            }

                            if let Some(host_p) = &gw.provider.resource_path {
                                let guest_p = gw.guest_resource_path().unwrap();
                                assets.insert(host_p.clone(), guest_p)?;
                            }

                            gazebo_containers.push(DeploymentContainer {
                                environment_variables,
                                assets,
                                generated_guest_files: Default::default(),
                                command: gw.container_command(),
                                args,
                                connections: connections.clone(),
                                components: vec![gw],
                            });
                        }
                    },
                    WorldOrMachineComponent::Machine(m) => match m.provider {
                        MachineProvider::Renode(p) => {
                            let mut rm = RenodeMachine {
                                guest_bin_shared: false,
                                base: m.base,
                                provider: p,
                            };

                            let found_conflicting_cli_configs = renode_container
                                .components
                                .iter()
                                .map(|r| &r.provider.cli)
                                .any(|cfg| *cfg != rm.provider.cli);
                            if found_conflicting_cli_configs {
                                todo!("Provider configs need to match up");
                            }

                            // Merge assets and env vars
                            renode_container.assets.merge(&rm.base.assets)?;
                            renode_container
                                .environment_variables
                                .merge(&rm.base.environment_variables)?;

                            // Add bin as guest asset file
                            if renode_container
                                .assets
                                .0
                                .insert(rm.base.bin.clone(), rm.guest_bin())
                                .is_some()
                            {
                                // Multiple machines on this container share this bin

                                // Remove the entry
                                renode_container.assets.0.remove(&rm.base.bin);

                                // Find the previous machine, set the shared path to true
                                renode_container
                                    .components
                                    .iter_mut()
                                    .filter(|m| m.base.bin == rm.base.bin)
                                    .for_each(|m| m.guest_bin_shared = true);

                                // Add the shared path
                                rm.guest_bin_shared = true;
                                renode_container
                                    .assets
                                    .0
                                    .insert(rm.base.bin.clone(), rm.guest_bin());
                            }

                            // Stuff only needed once
                            if renode_container.components.is_empty() {
                                renode_container.command = rm.container_command();
                                renode_container.args = rm.container_args();
                            }

                            renode_container.components.push(rm);
                        }
                        MachineProvider::Qemu(p) => {
                            let qm = QemuMachine {
                                base: m.base,
                                provider: p,
                            };

                            // Add bin path to assets
                            let mut assets = qm.base.assets.clone();
                            assets.0.insert(qm.base.bin.clone(), qm.guest_bin());

                            // Add guest bin path to args
                            let mut args = qm.container_args();
                            args.push(qm.guest_bin().display().to_string());

                            qemu_containers.push(DeploymentContainer {
                                environment_variables: qm.base.environment_variables.clone(),
                                assets,
                                generated_guest_files: Default::default(),
                                command: qm.container_command(),
                                args,
                                connections: connections.clone(),
                                components: vec![qm],
                            });
                        }
                        MachineProvider::Container(p) => {
                            let cm = ContainerMachine {
                                base: m.base,
                                provider: p,
                            };
                            // TODO
                            // add path/to/guest bin to assets and args
                            // whatever we need for this kind
                            container_containers.push(DeploymentContainer {
                                environment_variables: cm.base.environment_variables.clone(),
                                assets: cm.base.assets.clone(),
                                generated_guest_files: Default::default(),
                                command: Default::default(),
                                args: Default::default(),
                                connections: connections.clone(),
                                components: vec![cm],
                            });
                        }
                    },
                }
            }

            if provider == ProviderKind::Renode {
                debug_assert!(
                    !renode_container.components.is_empty(),
                    "Renode machines should not be empty"
                );

                renode_container
                    .args
                    .push(guest_resc_path().display().to_string());

                let mut resc_content = Vec::new();
                RenodeScriptGen::new(&mut resc_content)
                    .generate(&renode_container.components, &renode_container.connections)?;

                renode_container
                    .generated_guest_files
                    .insert(guest_resc_path(), str::from_utf8(&resc_content)?.to_owned());

                renode_containers.push(renode_container);
            }
        }

        if gazebo_containers
            .iter()
            .map(|c| c.components.len())
            .max()
            .unwrap_or(0)
            > 1
        {
            todo!("Gazebo container may only contain 1 world");
        }

        if qemu_containers
            .iter()
            .map(|c| c.components.len())
            .max()
            .unwrap_or(0)
            > 1
        {
            todo!("Qemu container may only contain 1 world");
        }

        // Each gazebo world gets a set of gazebo-specific env vars
        // synthesized and propagated to both the self world and
        // any immediately network-connected neighboring components
        for gz in gazebo_containers.iter_mut() {
            let gz_comp = gz.components.get(0).unwrap();
            let (partition_k, partition_v) = gz_comp.partition_env_kv();

            // Add to self
            gz.environment_variables
                .insert(partition_k.to_owned(), partition_v.clone())?;

            // Add to neighboring components connected to this
            // TODO - restrict by connection kind
            let gz_comp_name = gz_comp.base.name.clone().into();
            for neighboring_comp in graph.neighboring_components(&gz_comp_name) {
                let env_vars = match graph.component(&neighboring_comp)?.provider() {
                    ProviderKind::Gazebo => {
                        todo!("Multiple gazebo worlds on the same network not supported yet")
                    }
                    ProviderKind::Renode => {
                        let cont = renode_containers
                            .iter_mut()
                            .find(|c| {
                                c.components
                                    .iter()
                                    .any(|m| m.base.name.as_str() == neighboring_comp.as_str())
                            })
                            .unwrap();
                        &mut cont.environment_variables
                    }
                    ProviderKind::Qemu => {
                        let cont = qemu_containers
                            .iter_mut()
                            .find(|c| {
                                c.components
                                    .iter()
                                    .any(|m| m.base.name.as_str() == neighboring_comp.as_str())
                            })
                            .unwrap();
                        &mut cont.environment_variables
                    }
                    ProviderKind::Container => {
                        let cont = container_containers
                            .iter_mut()
                            .find(|c| {
                                c.components
                                    .iter()
                                    .any(|m| m.base.name.as_str() == neighboring_comp.as_str())
                            })
                            .unwrap();
                        &mut cont.environment_variables
                    }
                };
                env_vars.insert(partition_k.to_owned(), partition_v.clone())?;
            }
        }

        Ok(Self {
            gazebo_containers,
            renode_containers,
            qemu_containers,
            container_containers,
        })
    }
}
