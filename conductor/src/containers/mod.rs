use anyhow::{anyhow, Context, Result};
use bollard::{
    container,
    image::{BuildImageOptions, CreateImageOptions},
    models::{DeviceRequest, Mount, MountTypeEnum},
    Docker,
};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::default::Default;
use std::path::{Path, PathBuf};
use tracing::{instrument, trace};

type ContainerClient = Docker;

#[derive(Debug, Default)]
pub struct Container {
    name: Option<String>,
    image: Option<String>,
    containerfile: Option<PathBuf>,
    context: Option<PathBuf>,
    cmd: Option<Vec<String>>,
    mounts: Option<HashMap<String, String>>,
    env: Option<Vec<String>>,
    gpu_cap: bool,
}

pub enum ContainerState {
    Defined,
    Built,
    Running,
}

// builder-ish things
impl Container {
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
}

impl Container {
    pub fn new() -> Container {
        Default::default()
    }

    pub fn from_internal_image(image: &str) -> Container {
        let image_context_dir = {
            // HACK: only supports repo-local images
            let mut path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
            path.pop();
            path.push("images/");
            path.push(image);
            trace!("image context: {}", path.display());
            path
        };

        Self::new()
            .with_image(format!("conductor/{image}"))
            .with_context(image_context_dir)
    }

    async fn client(&self) -> ContainerClient {
        Docker::connect_with_local_defaults()
            .context("connect to container system service")
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
        trace!("get client");
        let client = self.client().await;
        trace!("got client");

        // TODO.pb: resolve system state to figure out if build needs to happen

        if self.containerfile.is_some() || self.context.is_some() {
            let image_options = BuildImageOptions {
                dockerfile: "Containerfile",
                t: &self.image.clone().unwrap_or_default(),
                labels: [
                    ("io.auxon.conductor", ""),
                    ("io.auxon.conductor.universe", "Foo"),
                    ("io.auxon.conductor.image-context-hash", "ABCDEF01"),
                ]
                .into(),
                ..Default::default()
            };

            let tarball = self.build_context_tar().await?;

            let mut build_image_progress =
                client.build_image(image_options, None, Some(tarball.into()));

            // receive progress reports on image being built
            while let Some(progress) = build_image_progress.next().await {
                //trace!(?response, "build image progress");
                if let Some(msg) = progress?.stream {
                    // TODO: print in the CLI handler
                    print!("{}", msg);
                }
            }

            trace!("image built");
        } else if let Some(ref image) = self.image {
            let image_options = CreateImageOptions {
                from_image: image.as_str(),
                ..Default::default()
            };

            trace!(?image_options, "create image");
            let mut create_image_progress = client.create_image(Some(image_options), None, None);

            // receive progress reports on image being built
            while let Some(progress) = create_image_progress.next().await {
                //trace!(?response, "build image progress");
                // TODO: print in the CLI handler
                print!("{:?}", progress);
            }
        }

        trace!("image created");

        // TODO: create container at build time

        Ok(())
    }

    #[instrument]
    pub async fn run(&mut self) -> Result<()> {
        let client = self.client().await;

        // TODO.pb: add metadata when creating container
        // TODO.pb: check if this is already running based on metadata of running containers
        // TODO.pb: move to build after ^

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

        let device_requests = self.gpu_cap.then_some({
            vec![DeviceRequest {
                capabilities: Some(vec![vec!["gpu".to_owned()]]),
                ..Default::default()
            }]
        });

        let cmd = self
            .cmd
            .as_ref()
            .map(|some_cmd| some_cmd.iter().map(|arg| arg.as_str()).collect());

        let container_config = container::Config {
            image,
            cmd,
            tty: Some(true),
            env,
            host_config: Some(bollard::models::HostConfig {
                device_requests,
                auto_remove: Some(true), // seems useful, maybe?
                // TODO - add real networking, expose host for easy mode for now
                network_mode: Some("host".to_owned()),
                mounts,
                ..Default::default()
            }),
            ..Default::default()
        };
        let container = client
            .create_container::<String, _>(
                self.name
                    .as_ref()
                    .map(|n| container::CreateContainerOptions {
                        name: n.clone(),
                        ..Default::default()
                    }),
                container_config,
            )
            .await?;

        trace!(?container, "created container");

        client
            .start_container::<String>(&container.id, None)
            .await?;

        Ok(())
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

        let mut container = Container::from_internal_image(IMAGE);

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_containerfile() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let containerfile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/Containerfile");

        let mut container = Container::new()
            .with_image(IMAGE)
            .with_containerfile(containerfile);

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_context() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let context = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/");

        let mut container = Container::new().with_image(IMAGE).with_context(context);

        container.build().await
    }

    #[tokio::test]
    #[traced_test]
    async fn command_in_container_from_image() -> Result<()> {
        const IMAGE: &str = "renode";

        let mut container = Container::from_internal_image(IMAGE).with_cmd(["whoami"]);

        container.build().await?;

        container.run().await
    }

    #[tokio::test]
    #[traced_test]
    async fn command_in_container_with_mount() -> Result<()> {
        const IMAGE: &str = "docker.io/ubuntu:latest";

        let c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/");
        let cc = c.canonicalize().unwrap();
        let sc = cc.as_os_str().to_str().unwrap();

        let mut container = Container::new()
            .with_image(IMAGE)
            .with_cmd(["/app/application.sh"])
            .with_mounts([(sc, "/app/")]);

        container.build().await?;

        container.run().await?;

        // TODO.pb: verify application ran automatically?

        // TODO.pb: `container.stop()` && `container.rm()` (or whatever) once that's a thing

        Ok(())
    }
}
