use crate::{
    containers::Container,
    provider::{
        container::ContainerMachine, gazebo::GazeboWorld, qemu::QemuMachine, renode::RenodeMachine,
    },
    ComponentGraph, Config, Deployment, DeploymentContainer, WorldOrMachineComponent,
};
use anyhow::Result;
use std::path::Path;

pub struct System {
    config: Config,
    containers: Vec<ContainerRuntime>,
}

impl System {
    pub fn from_config_no_runtime(config: Config) -> Self {
        System {
            config,
            containers: Vec::new(),
        }
    }

    pub fn from_config(config: Config) -> Result<Self> {
        let mut sys = Self::from_config_no_runtime(config);
        sys.build_runtime_containers_from_deployment()?;
        Ok(sys)
    }

    pub fn try_from_config_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = Config::read(path)?;
        Self::from_config(config)
    }

    pub fn try_from_working_directory() -> Result<Self> {
        let config_path = conductor_config::find_config_file()?;
        let config = Config::read(config_path)?;
        Self::from_config(config)
    }

    pub fn config(&self) -> &Config {
        &self.config
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
        let deployment = Deployment::from_graph(self.config.global.name.clone(), &graph)?;
        Ok(deployment)
    }

    pub fn build_runtime_containers_from_deployment(&mut self) -> Result<()> {
        debug_assert!(self.containers.is_empty());
        let deployment = self.deployment()?;
        for c in deployment.gazebo_containers.iter() {
            self.containers.push(ContainerRuntime::new_gazebo_world(c));
        }
        for c in deployment.renode_containers.iter() {
            self.containers
                .push(ContainerRuntime::new_renode_machine(c));
        }
        for c in deployment.qemu_containers.iter() {
            self.containers.push(ContainerRuntime::new_qemu_machine(c));
        }
        for c in deployment.container_containers.iter() {
            self.containers
                .push(ContainerRuntime::new_container_machine(c));
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
}

#[derive(Debug)]
struct ContainerRuntime {
    // TODO maybe flatten this if nothing else needed?
    // name: ContainerRuntimeName,
    // deployment: DeploymentContainer<C>
    // ...?
    container: Container,
}

impl ContainerRuntime {
    fn new_gazebo_world(deployment: &DeploymentContainer<GazeboWorld>) -> Self {
        let name = deployment.name.clone();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::new();
        container.set_name(name.as_str());
        container.set_image(deployment.world().base_image());
        container.set_cmd(cmd);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };
        Self { container }
    }

    fn new_renode_machine(deployment: &DeploymentContainer<RenodeMachine>) -> Self {
        let name = deployment.name.clone();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::new();
        container.set_name(name.as_str());
        container.set_image(deployment.base_image());
        container.set_cmd(cmd);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };
        Self { container }
    }

    fn new_qemu_machine(deployment: &DeploymentContainer<QemuMachine>) -> Self {
        let name = deployment.name.clone();
        let mut cmd = deployment.args.clone();
        cmd.insert(0, deployment.command.clone());
        let mut container = Container::new();
        container.set_name(name.as_str());
        container.set_image(deployment.machine().base_image());
        container.set_cmd(cmd);
        if !deployment.assets.is_empty() {
            let mounts = deployment
                .assets
                .as_ref()
                .iter()
                .map(|asset| (asset.0.to_str().unwrap(), asset.1.to_str().unwrap()));
            container.set_mounts(mounts);
        };
        Self { container }
    }

    fn new_container_machine(deployment: &DeploymentContainer<ContainerMachine>) -> Self {
        let name = deployment.name.clone();
        let machine = deployment.machine();

        // This is not great, not sure which way I want to fix this yet.
        let mut container = Container::new();
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
        Self { container }
    }

    pub(crate) async fn build(&mut self) -> Result<()> {
        self.container.build().await
    }

    pub(crate) async fn start(&mut self) -> Result<()> {
        self.container.run().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Global;
    use crate::types::SystemName;
    use std::collections::BTreeSet;

    #[test]
    fn get_system_from_config_path() -> Result<()> {
        System::try_from_config_path(
            "../test_resources/systems/single-container-machine/conductor.toml",
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn run_fake_system() -> Result<()> {
        let mut system = System {
            config: Config {
                global: Global {
                    name: SystemName::new_canonicalize("fake-system").unwrap(),
                    environment_variables: Default::default(),
                },
                machines: Vec::new(),
                connections: BTreeSet::new(),
                worlds: Vec::new(),
            },
            containers: vec![ContainerRuntime {
                container: Container::new().with_image("docker.io/ubuntu:latest"),
            }],
        };

        system.build().await?;

        system.start().await?;

        Ok(())
    }
}
