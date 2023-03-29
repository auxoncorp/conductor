use anyhow::{anyhow, Context, Result};
use bollard::{
    container,
    container::RemoveContainerOptions,
    exec::{CreateExecOptions, StartExecResults},
    image::{BuildImageOptions, CreateImageOptions},
    Docker,
};
use futures_util::{StreamExt, TryStreamExt};
use std::default::Default;
use std::path::PathBuf;
use tracing::{error, instrument, trace};

#[instrument]
pub async fn run_in_container(
    image: impl AsRef<str> + std::fmt::Debug,
    command: Vec<&str>,
) -> Result<()> {
    let client =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    let image_response = client
        .create_image(
            Some(CreateImageOptions {
                from_image: image.as_ref(),
                ..Default::default()
            }),
            None,
            None,
        )
        .try_collect::<Vec<_>>()
        .await
        .context("fetch image from remote container registry")?;

    trace!(?image_response, "created image");

    let container_config = container::Config {
        image: Some(image.as_ref()),
        tty: Some(true),
        ..Default::default()
    };
    let container = client
        .create_container::<&str, &str>(None, container_config)
        .await?;

    trace!(?container, "created container");

    client
        .start_container::<String>(&container.id, None)
        .await?;

    trace!("started container");

    let exec = client
        .create_exec(
            &container.id,
            CreateExecOptions {
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                cmd: Some(command),
                ..Default::default()
            },
        )
        .await?;

    trace!(?exec, "container exec created");

    if let StartExecResults::Attached {
        mut output,
        input: _,
    } = client.start_exec(&exec.id, None).await?
    {
        trace!("container exec started");
        while let Some(Ok(msg)) = output.next().await {
            print!("{msg}");
        }
    } else {
        error!("failed to attach to container");
        panic!();
    }

    client
        .remove_container(
            &container.id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;

    Ok(())
}

#[instrument]
pub async fn build_image_from_name(image: impl AsRef<str> + std::fmt::Debug) -> Result<()> {
    let client =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    let image_response = client
        .create_image(
            Some(CreateImageOptions {
                from_image: image.as_ref(),
                ..Default::default()
            }),
            None,
            None,
        )
        .try_collect::<Vec<_>>()
        .await
        .context("fetch image from remote container registry")?;

    trace!(?image_response, "created image");

    let container_config = container::Config {
        image: Some(image.as_ref()),
        tty: Some(true),
        ..Default::default()
    };
    let container = client
        .create_container::<&str, &str>(None, container_config)
        .await?;

    trace!(?container, "created container");

    Ok(())
}

#[instrument]
pub async fn build_official_local_image(name: &str) -> Result<()> {
    let image_context_dir = {
        // HACK: only supports repo-local images
        let mut path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        path.pop();
        path.push("images/");
        path.push(name);
        trace!("image context: {}", path.display());
        path
    };

    let image_name = format!("conductor/{}", name);

    build_image_from_context(&image_name, image_context_dir).await
}

#[instrument]
pub async fn build_image_from_containerfile(name: &str, containerfile: PathBuf) -> Result<()> {
    let tarball_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        // TODO: stream tarball
        let mut tarball = tar::Builder::new(Vec::new());

        let containerfile_file_name = containerfile
            .file_name()
            .ok_or(anyhow!("containerfile does not name a file"))?
            .to_os_string();

        tarball
            .append_path_with_name(containerfile, containerfile_file_name)
            .context("build in-memory image tarball")
            .unwrap();

        tarball
            .into_inner()
            .context("finish in-memory image tarball")
    })
    .await
    .context("spawn blocking tokio task to build tarball")??;

    let image_name = format!("conductor/{}", name);

    build_image_from_tar(&image_name, tarball_bytes).await
}

#[instrument]
pub async fn build_image_from_context(name: &str, context: PathBuf) -> Result<()> {
    trace!(?context, "build image");
    let tarball_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        // TODO: stream tarball
        let mut tarball = tar::Builder::new(Vec::new());
        tarball
            .append_dir_all(".", context)
            .context("build in-memory image tarball")
            .unwrap();

        tarball
            .into_inner()
            .context("finish in-memory image tarball")
    })
    .await
    .context("spawn blocking tokio task to build tarball")??;

    let image_name = format!("conductor/{}", name);

    build_image_from_tar(&image_name, tarball_bytes).await
}

#[instrument(skip(tarball))]
pub async fn build_image_from_tar(name: &str, tarball: Vec<u8>) -> Result<()> {
    let client =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    let image_options = BuildImageOptions {
        dockerfile: "Containerfile",
        t: name,
        labels: [
            ("io.auxon.conductor", ""),
            ("io.auxon.conductor.universe", "Foo"),
            ("io.auxon.conductor.image-context-hash", "ABCDEF01"),
        ]
        .into(),
        ..Default::default()
    };

    let mut build_image_progress = client.build_image(image_options, None, Some(tarball.into()));

    // receive progress reports on image being built
    while let Some(progress) = build_image_progress.next().await {
        //trace!(?response, "build image progress");
        if let Some(msg) = progress?.stream {
            // TODO: print in the CLI handler
            print!("{}", msg);
        }
    }

    trace!("image built");

    Ok(())
}

#[instrument]
pub async fn start_container_from_image(image: &str) -> Result<()> {
    let client =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    // TODO: do create in build, depends on somehow associating containers between runs
    let container_config = container::Config {
        image: Some(image),
        tty: Some(true),
        ..Default::default()
    };
    let container = client
        .create_container::<&str, &str>(None, container_config)
        .await?;

    trace!(?container, "created container");

    client
        .start_container::<String>(&container.id, None)
        .await?;

    trace!("container started");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn find_image() -> Result<()> {
        use bollard::image::SearchImagesOptions;

        const IMAGE: &str = "debian";
        const TAG: &str = "bullseye";

        let client = Docker::connect_with_local_defaults()?;

        let images = client.list_images::<&str>(None).await?;
        trace!("local images: {images:#?}");

        let image = client.inspect_image("ubuntu1").await;
        trace!("ubuntu image: {image:#?}");

        let search_options = SearchImagesOptions {
            term: "docker.io/rust",
            ..Default::default()
        };
        let searched_images = client.search_images(search_options).await;
        trace!("searched images: {searched_images:#?}");

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn hello_world() -> Result<()> {
        const IMAGE: &str = "docker.io/ubuntu:latest";

        run_in_container(IMAGE, vec!["uname", "-a"]).await
    }

    #[tokio::test]
    #[traced_test]
    async fn official_image() -> Result<()> {
        const IMAGE: &str = "renode";

        build_official_local_image(IMAGE).await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_containerfile() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let containerfile = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/Containerfile");

        build_image_from_containerfile(IMAGE, containerfile).await
    }

    #[tokio::test]
    #[traced_test]
    async fn image_from_context() -> Result<()> {
        const IMAGE: &str = "single-container-machine";

        let context = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test_resources/systems/single-container-machine/");

        build_image_from_context(IMAGE, context).await
    }
}
