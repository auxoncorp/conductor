use anyhow::{anyhow, bail, Context as _, Result};
use bollard::{
    container::{self, AttachContainerOptions, ListContainersOptions},
    image::{BuildImageOptions, CreateImageOptions, ListImagesOptions},
    models::{DeviceMapping, DeviceRequest, EndpointSettings, Mount, MountTypeEnum},
    Docker,
};
use data_encoding::HEXLOWER;
use futures_util::StreamExt;
use ring::digest::{Context, Digest, SHA256};
use std::collections::HashMap;
use std::default::Default;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{info, instrument, trace, warn};

pub mod network;

pub use network::{Network, NetworkState};

// TODO: use a real local type
pub use bollard::container::LogOutput;

type ContainerClient = Docker;

#[derive(Debug, Default)]
pub struct ContainerBuilder {
    name: Option<String>,
    image: Option<String>,
    containerfile: Option<PathBuf>,
    context: Option<PathBuf>,
    cmd: Option<Vec<String>>,
    mounts: Option<HashMap<String, String>>,
    env: Option<Vec<String>>,
    gpu_cap: bool,
    networks: Vec<Network>,
}

#[derive(Debug, Default)]
pub struct Container {
    name: Option<String>,
    state: ContainerState,
    image: Option<String>,
    containerfile: Option<PathBuf>,
    containerfile_digest: Option<Digest>,
    context: Option<PathBuf>,
    context_digest: Option<Digest>,
    cmd: Option<Vec<String>>,
    mounts: Option<HashMap<String, String>>,
    env: Option<Vec<String>>,
    gpu_cap: bool,
    networks: Vec<Network>,
}

#[derive(Debug, Default)]
pub enum ContainerState {
    #[default]
    Defined,
    Built {
        image_id: String,
        container_id: String,
    },
    Running {
        image_id: String,
        container_id: String,
    },
}

// builder-ish things
impl ContainerBuilder {
    pub fn set_name(&mut self, name: impl AsRef<str>) {
        self.name = Some(name.as_ref().to_string());
    }
    pub fn with_name(mut self, name: impl AsRef<str>) -> Self {
        self.set_name(name);

        self
    }

    pub fn set_image(&mut self, image: impl AsRef<str>) {
        self.image = Some(image.as_ref().to_string());
    }
    pub fn with_image(mut self, image: impl AsRef<str>) -> Self {
        self.set_image(image);

        self
    }

    pub fn set_containerfile(&mut self, containerfile: impl AsRef<Path>) {
        self.containerfile = Some(containerfile.as_ref().to_path_buf());
    }
    pub fn with_containerfile(mut self, containerfile: impl AsRef<Path>) -> Self {
        self.set_containerfile(containerfile);

        self
    }

    pub fn set_context(&mut self, context: impl AsRef<Path>) {
        self.context = Some(context.as_ref().to_path_buf());
    }
    pub fn with_context(mut self, context: impl AsRef<Path>) -> Self {
        self.set_context(context);

        self
    }

    pub fn set_cmd(&mut self, cmd: impl IntoIterator<Item = impl AsRef<str>>) {
        self.cmd = Some(cmd.into_iter().map(|a| a.as_ref().to_string()).collect());
    }
    pub fn with_cmd(mut self, cmd: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.set_cmd(cmd);

        self
    }

    pub fn set_mounts(
        &mut self,
        mounts: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) {
        let mounts = HashMap::from_iter(
            mounts
                .into_iter()
                .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string())),
        );
        self.mounts = Some(mounts);
    }
    pub fn with_mounts(
        mut self,
        mounts: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Self {
        self.set_mounts(mounts);

        self
    }

    pub fn set_env(&mut self, env: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>) {
        let env = Vec::from_iter(
            env.into_iter()
                .map(|(var, val)| format!("{}={}", var.as_ref(), val.as_ref())),
        );
        self.env = Some(env);
    }
    pub fn with_env(
        mut self,
        env: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Self {
        self.set_env(env);

        self
    }

    pub fn set_gpu_cap(&mut self, gpu_cap: bool) {
        self.gpu_cap = gpu_cap;
    }
    pub fn with_gpu_cap(mut self, gpu_cap: bool) -> Self {
        self.set_gpu_cap(gpu_cap);

        self
    }

    pub fn set_networks(&mut self, networks: Vec<Network>) {
        self.networks = networks;
    }
    pub fn with_networks(mut self, networks: Vec<Network>) -> Self {
        self.set_networks(networks);

        self
    }

    pub async fn resolve(self) -> Result<Container> {
        let client = Docker::connect_with_local_defaults()
            .context("connect to container system service")?
            .negotiate_version()
            .await?;

        trace!("negotiated version: {}", client.client_version());

        // compile shared filters
        let mut filters = Vec::new();

        if let Some(ref image) = self.image {
            trace!("io.auxon.conductor.image = {image}");

            filters.push(format!("io.auxon.conductor.name={}", image));
        }

        let containerfile_digest = if let Some(ref containerfile) = self.containerfile {
            let containerfile_digest = digest_for_path(containerfile)?;
            let containerfile_digest_str = HEXLOWER.encode(containerfile_digest.as_ref());

            trace!("io.auxon.conductor.containerfile = {containerfile_digest_str}");

            filters.push(format!(
                "io.auxon.conductor.containerfile={}",
                containerfile_digest_str,
            ));

            Some(containerfile_digest)
        } else {
            None
        };

        let context_digest = if let Some(ref context) = self.context {
            let context_digest = digest_for_path(context)?;
            let context_digest_str = HEXLOWER.encode(context_digest.as_ref());

            trace!("io.auxon.conductor.context = {context_digest_str}");

            filters.push(format!("io.auxon.conductor.context={}", context_digest_str,));

            Some(context_digest)
        } else {
            None
        };

        // lookup existing local image, if it exists
        let image_id =
            if self.image.is_some() && self.context.is_none() && self.containerfile.is_none() {
                let image_resp = client.inspect_image(self.image.as_deref().unwrap()).await;

                image_resp.map(|image| image.id).ok().flatten()
            } else {
                let images = client
                    .list_images(Some(ListImagesOptions {
                        filters: HashMap::from_iter([("label".to_string(), filters.clone())]),
                        ..Default::default()
                    }))
                    .await?;

                images.into_iter().next().map(|i| i.id)
            };

        //trace!("image: {image_id:?}");

        // lookup existing built container, if it exists
        let containers = client
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters: HashMap::from_iter([("label".to_string(), filters)]),
                ..Default::default()
            }))
            .await?;

        //trace!("containers: {containers:#?}");

        let state = if let (Some(image), Some(container)) = (image_id, containers.get(0)) {
            if container.state == Some("running".to_string()) {
                ContainerState::Running {
                    image_id: image,
                    container_id: container.id.clone().expect("container that exists has id"),
                }
            } else {
                ContainerState::Built {
                    image_id: image,
                    container_id: container.id.clone().expect("container that exists has id"),
                }
            }
        } else {
            //let images = client.list_images::<&str>(None).await?;
            //trace!("all images: {images:#?}");
            //let containers = client.list_containers::<&str>(None).await?;
            //trace!("all containers: {containers:#?}");

            //panic!("it shouldn't be");

            ContainerState::Defined
        };

        //// inspect *local* images
        //let image = client.inspect_image("ubuntu1").await;
        //trace!("ubuntu image: {image:#?}");

        //// Searches *remote* images
        //let search_options = SearchImagesOptions {
        //    term: "docker.io/rust",
        //    ..Default::default()
        //};
        //let searched_images = client.search_images(search_options).await;
        //trace!("searched images: {searched_images:#?}");

        let container = Container {
            name: self.name,
            state,
            image: self.image,
            containerfile: self.containerfile,
            containerfile_digest,
            context: self.context,
            context_digest,
            cmd: self.cmd,
            mounts: self.mounts,
            env: self.env,
            gpu_cap: self.gpu_cap,
            networks: self.networks,
        };

        Ok(container)
    }
}

#[instrument]
fn digest_for_path(path: &Path) -> Result<Digest> {
    let mut context = Context::new(&SHA256);

    if path.is_dir() {
        digest_dir(&mut context, path, path)?;
    } else if path.is_file() {
        digest_file(&mut context, path, path)?;
    } else {
        bail!("path is not a directory or file: {}", path.display());
    }

    Ok(context.finish())
}

#[instrument(skip(context))]
fn digest_dir(context: &mut Context, dir: &Path, ref_dir: &Path) -> Result<()> {
    // list dir
    let mut paths = fs::read_dir(dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;

    // sort listing
    paths.sort();

    // digest each file
    for ref path in paths {
        // TODO: do I need to prevent infinite recursions? What does the tar library do?
        if path.is_dir() {
            digest_dir(context, path, ref_dir)?;
        } else if path.is_file() {
            digest_file(context, path, ref_dir)?;
        } else {
            bail!("path is not a directory or file");
        }
    }

    Ok(())
}

#[instrument(skip(context))]
fn digest_file(context: &mut Context, file: &Path, ref_dir: &Path) -> Result<()> {
    // feed in releative (to context path) file path
    let rel_path = file.strip_prefix(ref_dir).context("get relative path")?;
    context.update(rel_path.to_string_lossy().as_bytes());

    // feed in file contents
    let input = File::open(file)?;
    let mut reader = BufReader::new(input);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Ok(())
}

impl Container {
    pub fn builder() -> ContainerBuilder {
        Default::default()
    }

    pub fn from_internal_image(image: &str) -> ContainerBuilder {
        let image_context_dir = {
            // HACK: only supports repo-local images
            let mut path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
            path.pop();
            path.push("images/");
            path.push(image);
            trace!("image context: {}", path.display());
            path
        };

        Self::builder()
            .with_image(format!("conductor/{image}"))
            .with_context(image_context_dir)
    }

    // TODO: should name just be made non-optional?
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    async fn client(&self) -> ContainerClient {
        Docker::connect_with_local_defaults()
            .context("connect to container system service")
            .unwrap()
            .negotiate_version()
            .await
            .unwrap()
    }

    async fn build_context_tar(&mut self) -> Result<Vec<u8>> {
        let containerfile = self.containerfile.clone();
        let context = self.context.clone();

        // TODO: stream files from FS, taring in flight, don't block
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let mut tarball = tar::Builder::new(Vec::new());

            if let Some(containerfile) = containerfile {
                let containerfile_file_name = containerfile
                    .file_name()
                    .ok_or(anyhow!("containerfile does not name a file"))?
                    .to_os_string();

                tarball
                    .append_path_with_name(containerfile, containerfile_file_name)
                    .context("build in-memory image tarball")
                    .unwrap();
            }

            if let Some(context) = context {
                tarball
                    .append_dir_all(".", context)
                    .context("build in-memory image tarball")
                    .unwrap();
            }

            // TODO: decide what should happen if `containerfile` is set and `context` has
            // containerfile

            tarball
                .into_inner()
                .context("finish in-memory image tarball")
        })
        .await
        .context("spawn blocking tokio task to build tarball")?
    }

    #[instrument]
    pub async fn build(&mut self) -> Result<()> {
        let client = self.client().await;

        match &self.state {
            ContainerState::Defined => {
                let mut labels = HashMap::new();

                // always apply this label to everything for easy filtering of all resources
                // TODO: use this for something? tool version? is that useful?
                labels.insert("io.auxon.conductor", "".into());

                if let Some(ref image) = self.image {
                    labels.insert("io.auxon.conductor.name", image.clone());
                }

                if let Some(containerfile_digest) = self.containerfile_digest {
                    labels.insert(
                        "io.auxon.conductor.containerfile",
                        HEXLOWER.encode(containerfile_digest.as_ref()),
                    );
                }

                if let Some(context_digest) = self.context_digest {
                    labels.insert(
                        "io.auxon.conductor.context",
                        HEXLOWER.encode(context_digest.as_ref()),
                    );
                }

                let labels_ref = labels.iter().map(|(k, v)| (*k, v.as_str())).collect();

                let image_id = if self.containerfile.is_some() || self.context.is_some() {
                    let image = self.image.clone().unwrap();
                    let image_options = BuildImageOptions {
                        dockerfile: "Containerfile",
                        t: &image,
                        labels: labels_ref,
                        ..Default::default()
                    };

                    let tarball = self.build_context_tar().await?;

                    let mut build_image_progress =
                        client.build_image(image_options, None, Some(tarball.into()));

                    // receive progress reports on image being built
                    let mut image_id = None;
                    while let Some(progress) = build_image_progress.next().await {
                        trace!(?progress, "build image progress");
                        let progress = progress?;
                        if let Some(msg) = progress.stream {
                            // TODO: print in the CLI handler
                            print!("{}", msg);
                        }

                        if let Some(aux) = progress.aux {
                            if let Some(id) = aux.id {
                                info!("id: {id}");
                                let _ = image_id.insert(id);
                            }
                        }
                    }

                    let Some(image_id) = image_id else {
                        bail!("no image id reported by container service");
                    };

                    trace!(image_id, "image built");

                    image_id
                } else if let Some(ref image) = self.image {
                    let image_options = CreateImageOptions {
                        from_image: image.as_str(),
                        ..Default::default()
                    };

                    trace!(?image_options, "create image");
                    let mut create_image_progress =
                        client.create_image(Some(image_options), None, None);

                    // receive progress reports on image being built
                    let mut image_id = None;
                    while let Some(progress) = create_image_progress.next().await {
                        //trace!(?progress, "create image progress");
                        // TODO: print in the CLI handler
                        let progress = progress?;
                        if let Some(msg) = progress.progress {
                            // TODO: print in the CLI handler
                            print!("{}", msg);
                        }

                        if let Some(id) = progress.id {
                            if id.starts_with("sha256:") {
                                info!("id: {id}");
                                let _ = image_id.insert(id);
                            } else {
                                warn!("non-id returned as ID: {id}");
                            }
                        }
                    }

                    // workaround: docker just reports the label (eg. "latest") as the "id", if
                    // that's the case, just copy over the source image name
                    let image_id = image_id.unwrap_or_else(|| image.to_string());

                    trace!(image_id, "image created");

                    image_id
                } else {
                    bail!("container without image definition")
                };

                // build network endpoint definition
                let mut container_network_endpoints = HashMap::new();
                for network in &self.networks {
                    let network_name = &network.name;

                    let NetworkState::Built { id: network_id } = &network.state else {
                            panic!("unbuilt network passed to container builder")
                        };

                    container_network_endpoints.insert(
                        network_name.as_str(),
                        EndpointSettings {
                            network_id: Some(network_id.to_string()),
                            // TODO: add alias for the simple machine name too, currently does the
                            // "fully qualified" name only, eg. "two_networked_containers___server"
                            aliases: self.name.clone().map(|n| vec![n]),
                            ..Default::default()
                        },
                    );
                }

                let image = self.image.as_deref();

                let env = self
                    .env
                    .as_ref()
                    .map(|vars| vars.iter().map(|ev| ev.as_str()).collect());

                let mounts = self.mounts.as_ref().map(|some_mounts| {
                    some_mounts
                        .iter()
                        .map(|(host_path, container_path)| Mount {
                            source: Some(host_path.as_str().to_string()),
                            target: Some(container_path.as_str().to_string()),
                            typ: Some(MountTypeEnum::BIND),
                            ..Default::default()
                        })
                        .collect()
                });

                let cmd = self
                    .cmd
                    .as_ref()
                    .map(|some_cmd| some_cmd.iter().map(|arg| arg.as_str()).collect());

                let labels_ref = labels.iter().map(|(k, v)| (*k, v.as_str())).collect();

                trace!(?container_network_endpoints);

                // hook up GPU for GUI containers
                let (devices, device_requests) = if self.gpu_cap {
                    if std::env::var("NVIDIA_GPU").is_ok() {
                        (
                            None,
                            Some(vec![DeviceRequest {
                                capabilities: Some(vec![vec!["gpu".to_owned()]]),
                                ..Default::default()
                            }]),
                        )
                    } else {
                        (
                            Some(vec![DeviceMapping {
                                path_on_host: Some("/dev/dri".to_string()),
                                path_in_container: Some("/dev/dri".to_string()),
                                cgroup_permissions: Some("rwm".to_string()),
                            }]),
                            None,
                        )
                    }
                } else {
                    (None, None)
                };

                let container_config = container::Config {
                    image,
                    cmd,
                    tty: Some(true),
                    env,
                    host_config: Some(bollard::models::HostConfig {
                        network_mode: Some("host".to_owned()),
                        // TODO - only need CAP_NET_ADMIN if dealing with TUN/TAP interfaces
                        //cap_add: Some(vec!["NET_ADMIN".to_owned()]),
                        mounts,
                        devices,
                        device_requests,
                        ..Default::default()
                    }),
                    labels: Some(labels_ref),
                    // networking must be set here explicitly to implicitly disable the default
                    // bridge network
                    //
                    // TODO: reenable networking, need to figure out how to make GUI work without
                    //       host network mode
                    //
                    //networking_config: Some(NetworkingConfig {
                    //    endpoints_config: container_network_endpoints,
                    //}),
                    ..Default::default()
                };

                let container = client
                    .create_container::<&str, _>(
                        None, // disabled until deleting works
                        /*self.name
                        .as_ref()
                        .map(|n| container::CreateContainerOptions {
                            name: n.clone(),
                            ..Default::default()
                        })*/
                        container_config,
                    )
                    .await?;

                trace!(?container, "created container");

                self.state = ContainerState::Built {
                    image_id,
                    container_id: container.id,
                };
            }
            ContainerState::Built { .. } => {
                trace!("image already built, nothing to build");
            }
            ContainerState::Running { .. } => {
                trace!("container already running, nothing to build");
            }
        }

        Ok(())
    }

    #[instrument]
    pub async fn start(&mut self) -> Result<()> {
        let client = self.client().await;

        match &self.state {
            ContainerState::Defined => {
                // TODO: just do the build here
                bail!("can't start unbuilt system");
            }
            ContainerState::Built {
                container_id,
                image_id,
            } => {
                trace!(container_id, "start previously built container");
                assert!(
                    !container_id.is_empty(),
                    "container id can't be the empty string"
                );
                client.start_container::<String>(container_id, None).await?;

                self.state = ContainerState::Running {
                    container_id: container_id.clone(),
                    image_id: image_id.clone(),
                };
            }
            ContainerState::Running { .. } => {
                trace!("container already running, nothing to do");
            }
        }

        Ok(())
    }

    #[instrument]
    pub async fn attach(&self) -> Result<bollard::container::AttachContainerResults> {
        let client = self.client().await;

        match &self.state {
            ContainerState::Defined => {
                bail!("machine not built or running, can't attach");
            }
            ContainerState::Built { .. } => {
                bail!("machine not running, can't attach");
            }
            ContainerState::Running { container_id, .. } => {
                trace!(container_id, "attach to container");
                let io = client
                    .attach_container::<String>(
                        container_id,
                        Some(AttachContainerOptions {
                            logs: Some(true),
                            stream: Some(true),
                            stdin: Some(true),
                            stdout: Some(true),
                            stderr: Some(true),
                            detach_keys: Some("ctrl-d".to_string()),
                        }),
                    )
                    .await?;

                Ok(io)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn internal_image() -> Result<()> {
        const IMAGE: &str = "renode";

        let mut container = Container::from_internal_image(IMAGE).resolve().await?;

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_containerfile() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let containerfile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/Containerfile");

        let mut container = Container::builder()
            .with_image(IMAGE)
            .with_containerfile(containerfile)
            .resolve()
            .await?;

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_context() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let context = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/");

        let mut container = Container::builder()
            .with_image(IMAGE)
            .with_context(context)
            .resolve()
            .await?;

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn command_in_container_from_image() -> Result<()> {
        const IMAGE: &str = "renode";

        let mut container = Container::from_internal_image(IMAGE)
            .with_cmd(["whoami"])
            .resolve()
            .await?;

        info!("build");
        container.build().await?;

        info!("run");
        container.start().await
    }

    #[tokio::test]
    #[traced_test]
    async fn command_in_container_with_mount() -> Result<()> {
        const IMAGE: &str = "docker.io/ubuntu:latest";

        let c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/");
        let cc = c.canonicalize().unwrap();
        let sc = cc.as_os_str().to_str().unwrap();

        let mut container = Container::builder()
            .with_image(IMAGE)
            .with_cmd(["/app/application.sh"])
            .with_mounts([(sc, "/app/")])
            .resolve()
            .await?;

        container.build().await?;

        container.start().await?;

        // TODO.pb: verify application ran automatically?

        // TODO.pb: `container.stop()` && `container.rm()` (or whatever) once that's a thing

        Ok(())
    }
}
