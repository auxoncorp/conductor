use crate::{
    config::{Connection, Global, MachineProvider, WorldProvider},
    display,
    provider::{
        container::ContainerMachine,
        gazebo::GazeboWorld,
        guest_component_resource_path,
        qemu::QemuMachine,
        renode::{self, guest_resc_path, PlatformDescription, RenodeMachine, RenodeScriptGen},
    },
    types::{
        BridgeName, ComponentName, ConnectionKind, ConnectionName, ContainerRuntimeName,
        EnvironmentVariableKeyValuePairs, HostToGuestAssetPaths, InterfaceName, ProviderKind,
        SystemName, TapDevice,
    },
    Component, ComponentGraph, WorldOrMachineComponent,
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    str,
};

// TODO Error type with contextual variants probably

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct DeploymentContainer<C> {
    pub name: ContainerRuntimeName,
    pub uses_host_display: bool,
    pub environment_variables: EnvironmentVariableKeyValuePairs,
    pub assets: HostToGuestAssetPaths,
    pub generated_guest_files: BTreeMap<PathBuf, String>,
    pub command: String,
    pub args: Vec<String>,
    pub connections: Vec<Connection>,
    pub taps_to_bridges: BTreeMap<TapDevice, BridgeName>,
    pub components: Vec<C>,
}

impl<C> DeploymentContainer<C> {
    fn empty(name: ContainerRuntimeName) -> Self {
        DeploymentContainer {
            name,
            uses_host_display: false,
            environment_variables: Default::default(),
            assets: Default::default(),
            generated_guest_files: Default::default(),
            command: Default::default(),
            args: Default::default(),
            connections: Vec::new(),
            taps_to_bridges: Default::default(),
            components: Vec::new(),
        }
    }
}

// NOTE: already sanity checked
impl DeploymentContainer<GazeboWorld> {
    pub(crate) fn world(&self) -> &GazeboWorld {
        self.components.get(0).unwrap()
    }
}

// NOTE: already sanity checked for consistency across multi-machine-per-container setups
impl DeploymentContainer<RenodeMachine> {
    pub(crate) fn base_image(&self) -> String {
        self.components.get(0).unwrap().base_image()
    }
}

// NOTE: already sanity checked
impl DeploymentContainer<QemuMachine> {
    pub(crate) fn machine(&self) -> &QemuMachine {
        self.components.get(0).unwrap()
    }
}

// NOTE: already sanity checked
impl DeploymentContainer<ContainerMachine> {
    pub(crate) fn machine(&self) -> &ContainerMachine {
        self.components.get(0).unwrap()
    }
}

#[derive(Debug)]
pub struct Deployment {
    pub system_name: SystemName,
    pub gazebo_containers: Vec<DeploymentContainer<GazeboWorld>>,
    pub renode_containers: Vec<DeploymentContainer<RenodeMachine>>,
    pub qemu_containers: Vec<DeploymentContainer<QemuMachine>>,
    pub container_containers: Vec<DeploymentContainer<ContainerMachine>>,
    pub wired_networks: BTreeSet<ConnectionName>,
}

impl Deployment {
    pub fn from_graph(
        global_config: &Global,
        graph: &ComponentGraph<WorldOrMachineComponent>,
    ) -> Result<Self> {
        let mut gazebo_containers = Vec::new();
        let mut renode_containers = Vec::new();
        let mut qemu_containers = Vec::new();
        let mut container_containers = Vec::new();

        let system_name = global_config.name.clone();

        let wired_networks = graph
            .connections()
            .iter()
            .filter_map(|(name, c)| {
                if c.kind() == ConnectionKind::Network {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for container in graph.components_by_container().iter() {
            // Renode provider can have multiple machines so its fields can be merged
            // as we iterator over each machine
            let rc_placeholder_name = ContainerRuntimeName::new_single(
                &system_name,
                container.components.iter().next().unwrap(), // TODO
            );
            let mut renode_container: DeploymentContainer<RenodeMachine> =
                DeploymentContainer::empty(rc_placeholder_name);

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

                            if let Some(host_p) = &gw.provider.gui_config_path {
                                let guest_p = gw.guest_gui_config_path().unwrap();
                                assets.insert(host_p.clone(), guest_p)?;
                            }

                            gazebo_containers.push(DeploymentContainer {
                                name: ContainerRuntimeName::new_single(
                                    &system_name,
                                    component_name,
                                ),
                                uses_host_display: !gw.provider.headless.unwrap_or(true),
                                environment_variables,
                                assets,
                                generated_guest_files: Default::default(),
                                command: gw.container_command(),
                                args,
                                connections: connections.clone(),
                                taps_to_bridges: Default::default(),
                                components: vec![gw],
                            });
                        }
                    },
                    WorldOrMachineComponent::Machine(m) => match m.provider {
                        MachineProvider::Renode(p) => {
                            let platform_descriptions = p
                                .resc
                                .platform_descriptions
                                .iter()
                                .map(|pd| PlatformDescription::from_config(pd))
                                .collect::<Result<Vec<PlatformDescription>, _>>()?;

                            let mut rm = RenodeMachine {
                                guest_bin_shared: false,
                                base: m.base,
                                provider: p,
                                platform_descriptions,
                                // TODO - each network kind connection to a different
                                // container (or host) gets a tap device
                                // name is conn_name_tap or w/e
                                tap_devices: Default::default(),
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

                            for (idx, network_connection) in
                                connections.iter().filter(|c| c.is_network()).enumerate()
                            {
                                rm.tap_devices.insert(
                                    network_connection.name().clone(),
                                    format!("renode-tap{idx}"),
                                );
                            }

                            // Stuff only needed once
                            if renode_container.components.is_empty() {
                                renode_container.command = rm.container_command();
                                renode_container.args = rm.container_args();
                            }

                            if !rm.provider.cli.disable_xwt.unwrap_or(true) {
                                renode_container.uses_host_display = true;
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
                                name: ContainerRuntimeName::new_single(
                                    &system_name,
                                    component_name,
                                ),
                                uses_host_display: !qm.provider.no_graphic.unwrap_or(true),
                                environment_variables: qm.base.environment_variables.clone(),
                                assets,
                                generated_guest_files: Default::default(),
                                command: qm.container_command(),
                                args,
                                connections: connections.clone(),
                                taps_to_bridges: Default::default(),
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
                                name: ContainerRuntimeName::new_single(
                                    &system_name,
                                    component_name,
                                ),
                                uses_host_display: false, // TODO - surface a config field for this
                                environment_variables: cm.base.environment_variables.clone(),
                                assets: cm.base.assets.clone(),
                                generated_guest_files: Default::default(),
                                command: Default::default(),
                                args: Default::default(),
                                connections: connections.clone(),
                                taps_to_bridges: Default::default(),
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

                for rm in renode_container.components.iter() {
                    for pd in rm.platform_descriptions.iter() {
                        if let PlatformDescription::LocalFile(file_name, content) = pd {
                            let guest_path =
                                guest_component_resource_path(&rm.base.name).join(file_name);
                            renode_container
                                .generated_guest_files
                                .insert(guest_path, content.clone());
                        }
                    }
                }

                renode_container
                    .args
                    .push(guest_resc_path().display().to_string());

                let tap_devices: BTreeMap<ConnectionName, TapDevice> = renode_container
                    .components
                    .iter()
                    .flat_map(|m| m.tap_devices.clone().into_iter())
                    .collect();

                for (tap, br) in tap_devices
                    .iter()
                    .map(|(conn_name, tap_dev)| {
                        let connection_index_in_config = graph
                            .connections()
                            .iter()
                            .position(|c| c.0 == conn_name)
                            .unwrap();

                        (
                            tap_dev.clone(),
                            InterfaceName::new_system_wired_network(connection_index_in_config),
                        )
                    })
                    .collect::<BTreeMap<TapDevice, BridgeName>>()
                {
                    renode_container.taps_to_bridges.insert(tap, br);
                }

                if !renode_container.taps_to_bridges.is_empty() {
                    let net_setup_guest_path = renode::guest_external_network_setup_script_path();
                    let net_setup_content = renode::external_network_setup_script_content(
                        &renode_container.taps_to_bridges,
                    );
                    renode_container
                        .generated_guest_files
                        .insert(net_setup_guest_path.clone(), net_setup_content);

                    let net_teardown_guest_path =
                        renode::guest_external_network_teardown_script_path();
                    let net_teardown_content = renode::external_network_teardown_script_content(
                        &renode_container.taps_to_bridges,
                    );
                    renode_container
                        .generated_guest_files
                        .insert(net_teardown_guest_path.clone(), net_teardown_content);

                    // When we have tap/bridge scripts, we need to change the runtime
                    // command and args to call them
                    // We convert '<cmd> <args>' into
                    // 'bash -c "net_setup.sh ; <cmd> <args> ; net_teardown.sh"'
                    let mut wrapped_args: Vec<String> =
                        std::iter::once(renode_container.command.clone())
                            .chain(renode_container.args.iter().cloned())
                            .collect();
                    wrapped_args.insert(0, net_setup_guest_path.display().to_string());
                    wrapped_args.insert(1, ";".to_owned());
                    wrapped_args.push(";".to_owned());
                    wrapped_args.push(net_teardown_guest_path.display().to_string());

                    renode_container.command = "/bin/bash".to_owned();
                    renode_container.args.clear();
                    renode_container.args.push("-c".to_owned());
                    renode_container.args.push(wrapped_args.join(" "));
                }

                let mut resc_content = Vec::new();
                RenodeScriptGen::new(&mut resc_content).generate(
                    &renode_container.components,
                    &renode_container.connections,
                    &tap_devices,
                )?;

                renode_container
                    .generated_guest_files
                    .insert(guest_resc_path(), str::from_utf8(&resc_content)?.to_owned());

                let comp_names: BTreeSet<ComponentName> = renode_container
                    .components
                    .iter()
                    .cloned()
                    .map(|c| c.base.name.into())
                    .collect();

                renode_container.name = ContainerRuntimeName::new_multi(&system_name, &comp_names);

                // TODO - using a pseudo tempdir on the host for
                // ephemeral store of file generated for the guest so
                // they can be treated as normal assets for now
                //
                // TODO use system name as part of arbitration
                if !renode_container.generated_guest_files.is_empty() {
                    for (guest_path, content) in renode_container.generated_guest_files.iter() {
                        let file_name = guest_path.file_name().unwrap();
                        let host_dir = PathBuf::from("/tmp")
                            .join("conductor_generated_assets")
                            .join(system_name.as_str())
                            .join(renode_container.name.as_ref());
                        let host_path = host_dir.join(file_name);
                        fs::create_dir_all(&host_dir)?;

                        // TODO - don't need to make everything executable
                        #[cfg(unix)]
                        {
                            use std::io::Write;
                            use std::os::unix::fs::OpenOptionsExt;

                            let mut f = fs::OpenOptions::new()
                                .mode(0o777)
                                .write(true)
                                .create(true)
                                .truncate(true)
                                .open(&host_path)?;
                            f.write_all(content.as_bytes())?;
                        }

                        // TODO - we don't support windows yet
                        #[cfg(not(unix))]
                        {
                            fs::write(&host_path, content)?;
                        }

                        renode_container
                            .assets
                            .insert(host_path, guest_path.clone())?;
                    }
                }

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

        let at_least_one_uses_display = gazebo_containers
            .iter()
            .map(|c| c.uses_host_display)
            .chain(renode_containers.iter().map(|c| c.uses_host_display))
            .chain(qemu_containers.iter().map(|c| c.uses_host_display))
            .chain(container_containers.iter().map(|c| c.uses_host_display))
            .any(|uses_host_display| uses_host_display);

        if at_least_one_uses_display {
            // Setup xauth/display environement
            let display = global_config.display.as_ref().expect("DISPLAY is required");
            let xauthority = global_config
                .xauthority
                .as_ref()
                .expect("Xauthority is required");

            // We create a system-specific guest xauth file, as a ro asset
            let guest_xauth_path = display::system_guest_xauth_file_path(&system_name);
            let mut guest_xauth = fs::File::create(&guest_xauth_path)?;
            display::write_guest_xauth(display, xauthority, &mut guest_xauth)?;

            for (env, assets) in gazebo_containers
                .iter_mut()
                .filter_map(env_and_assets_for_gui_container)
                .chain(
                    renode_containers
                        .iter_mut()
                        .filter_map(env_and_assets_for_gui_container),
                )
                .chain(
                    qemu_containers
                        .iter_mut()
                        .filter_map(env_and_assets_for_gui_container),
                )
                .chain(
                    container_containers
                        .iter_mut()
                        .filter_map(env_and_assets_for_gui_container),
                )
            {
                env.insert(display::DISPLAY_ENV_VAR.to_owned(), display.clone())?;
                env.insert(
                    display::XAUTHORITY_ENV_VAR.to_owned(),
                    guest_xauth_path.display().to_string(),
                )?;

                assets.insert(
                    PathBuf::from(display::HOST_X11_DOMAIN_SOCKET),
                    PathBuf::from(format!("{}:ro", display::HOST_X11_DOMAIN_SOCKET)),
                )?;
                assets.insert(guest_xauth_path.clone(), guest_xauth_path.clone())?;
            }
        }

        Ok(Self {
            system_name,
            gazebo_containers,
            renode_containers,
            qemu_containers,
            container_containers,
            wired_networks,
        })
    }
}

fn env_and_assets_for_gui_container<C>(
    c: &mut DeploymentContainer<C>,
) -> Option<(
    &mut EnvironmentVariableKeyValuePairs,
    &mut HostToGuestAssetPaths,
)> {
    c.uses_host_display
        .then_some((&mut c.environment_variables, &mut c.assets))
}
