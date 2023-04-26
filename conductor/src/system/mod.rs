use crate::{
    component::Component,
    config::{ConnectorProperties, MachineConnector},
    containers::{Container, Network},
    provider::{
        container::ContainerMachine, gazebo::GazeboWorld, qemu::QemuMachine, renode::RenodeMachine,
    },
    types::{ConnectionName, ContainerRuntimeName},
    ComponentGraph, Config, Deployment, DeploymentContainer, WorldOrMachineComponent,
};
use anyhow::{bail, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub struct System {
    config: Config,
    containers: Vec<Container>,
    networks: BTreeMap<ConnectionName, Network>,
}

impl System {
    pub fn from_config_no_runtime(config: Config) -> Self {
        System {
            config,
            containers: Vec::new(),
            networks: BTreeMap::new(),
        }
    }

    pub async fn from_config(config: Config) -> Result<Self> {
        let mut sys = Self::from_config_no_runtime(config);
        sys.build_runtime_containers_from_deployment().await?;
        Ok(sys)
    }

    pub async fn try_from_config_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = Config::read(path)?;
        Self::from_config(config).await
    }

    pub async fn try_from_working_directory() -> Result<Self> {
        let config_path = conductor_config::find_config_file()?;
        let config = Config::read(config_path)?;
        Self::from_config(config).await
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn containers(&self) -> impl IntoIterator<Item = &Container> {
        self.containers.as_slice()
    }

    pub fn components(&self) -> Vec<WorldOrMachineComponent> {
        self.config
            .worlds
            .iter()
            .cloned()
            .map(WorldOrMachineComponent::from)
            .chain(
                self.config
                    .machines
                    .iter()
                    .cloned()
                    .map(WorldOrMachineComponent::from),
            )
            .collect()
    }

    pub fn graph(&self) -> Result<ComponentGraph<WorldOrMachineComponent>> {
        let components = self.components();
        let connections = self.config.connections.clone();
        let g = ComponentGraph::new(components, connections)?;
        Ok(g)
    }

    pub fn deployment(&self) -> Result<Deployment> {
        let graph = self.graph().unwrap();
        let deployment = Deployment::from_graph(&self.config.global, &graph)?;
        Ok(deployment)
    }

    pub fn container_runtime_name_for_machine_named(
        &self,
        machine: &str,
    ) -> Result<ContainerRuntimeName> {
        for known_machine in &self.config.machines {
            if known_machine.base.name.as_str() == machine {
                return Ok(ContainerRuntimeName::new_single(
                    &self.config.global.name,
                    &known_machine.name(),
                ));
            }
        }

        bail!("machine not found")
    }

    pub async fn build_runtime_containers_from_deployment(&mut self) -> Result<()> {
        debug_assert!(self.containers.is_empty());
        let deployment = self.deployment()?;

        for n in deployment.wired_networks.iter() {
            // TODO: find a way to identify networks by more than name
            self.networks.insert(
                n.clone(),
                Network::builder().name(n.to_string()).resolve().await?,
            );
        }

        for c in deployment.gazebo_containers.iter() {
            self.new_gazebo_world(c).await?;
        }
        for c in deployment.renode_containers.iter() {
            self.new_renode_machine(c).await?;
        }
        for c in deployment.qemu_containers.iter() {
            self.new_qemu_machine(c).await?;
        }
        for c in deployment.container_containers.iter() {
            self.new_container_machine(c).await?;
        }

        Ok(())
    }

    pub async fn build(&mut self) -> Result<()> {
        for rt in &mut self.containers {
            rt.build().await?;
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        for rt in &mut self.containers {
            rt.start().await?;
        }

        Ok(())
    }

    async fn new_gazebo_world(
        &mut self,
        deployment: &DeploymentContainer<GazeboWorld>,
    ) -> Result<()> {
        let name = deployment.name.clone();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::from_internal_image(&deployment.world().base_image())
            .with_name(name.as_str())
            .with_cmd(cmd)
            .with_env(&deployment.environment_variables.0)
            .with_gpu_cap(deployment.uses_host_display);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };

        // TODO: networks

        self.containers.push(container.resolve().await?);

        Ok(())
    }

    async fn new_renode_machine(
        &mut self,
        deployment: &DeploymentContainer<RenodeMachine>,
    ) -> Result<()> {
        let name = deployment.name.clone();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::from_internal_image(&deployment.base_image())
            .with_name(name.as_str())
            .with_cmd(cmd)
            .with_env(&deployment.environment_variables.0)
            .with_gpu_cap(deployment.uses_host_display);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };

        // TODO: networks

        self.containers.push(container.resolve().await?);

        Ok(())
    }

    async fn new_qemu_machine(
        &mut self,
        deployment: &DeploymentContainer<QemuMachine>,
    ) -> Result<()> {
        let name = deployment.name.clone();
        let machine = deployment.machine();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::from_internal_image(&machine.base_image())
            .with_name(name.as_str())
            .with_cmd(cmd)
            .with_env(&deployment.environment_variables.0)
            .with_gpu_cap(deployment.uses_host_display);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };
        let networks = machine
            .base
            .connectors
            .iter()
            .filter_map(|c| self.try_get_network_for_connection(c))
            .collect();
        container.set_networks(networks);

        self.containers.push(container.resolve().await?);

        Ok(())
    }

    async fn new_container_machine(
        &mut self,
        deployment: &DeploymentContainer<ContainerMachine>,
    ) -> Result<()> {
        let name = deployment.name.clone();
        let machine = deployment.machine();

        // This is not great, not sure which way I want to fix this yet.
        let mut container = Container::builder();
        container.set_name(name.as_str());
        if let Some(ref image) = machine.provider.image {
            container.set_image(image);
        };
        if let Some(ref containerfile) = machine.provider.containerfile {
            container.set_containerfile(containerfile);
        };
        if let Some(ref context) = machine.provider.context {
            container.set_context(context);
        };
        if !machine.base.assets.is_empty() {
            let mounts = machine
                .base
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };
        // TODO: get this from bin once bin is optional and plumbed through
        if let Some(ref cmd) = None::<Vec<String>> {
            container.set_cmd(cmd);
        };
        container.set_env(&deployment.environment_variables.0);

        let networks = machine
            .base
            .connectors
            .iter()
            .filter_map(|c| self.try_get_network_for_connection(c))
            .collect();
        container.set_networks(networks);

        self.containers.push(container.resolve().await?);

        Ok(())
    }

    fn try_get_network_for_connection(&self, conn: &MachineConnector) -> Option<Network> {
        match &conn.properties {
            ConnectorProperties::Network(_) => Some(self.networks.get(&conn.name)?.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Global;
    use crate::types::SystemName;
    use std::collections::BTreeSet;

    #[tokio::test]
    async fn get_system_from_config_path() -> Result<()> {
        System::try_from_config_path(
            "../test_resources/systems/single-container-machine/conductor.toml",
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn run_fake_system() -> Result<()> {
        let mut system = System {
            config: Config {
                global: Global {
                    name: SystemName::new_canonicalize("fake-system").unwrap(),
                    display: None,
                    xauthority: None,
                    environment_variables: Default::default(),
                },
                machines: Vec::new(),
                connections: BTreeSet::new(),
                worlds: Vec::new(),
            },
            containers: vec![
                Container::builder()
                    .with_image("docker.io/ubuntu:latest")
                    .resolve()
                    .await?,
            ],
            networks: BTreeMap::new(),
        };

        system.build().await?;

        system.start().await?;

        Ok(())
    }
}
