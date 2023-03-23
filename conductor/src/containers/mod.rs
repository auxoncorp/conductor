use anyhow::{Context, Result};
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
use tracing::{error, trace};

pub async fn run_in_container(image: impl AsRef<str>, command: Vec<&str>) -> Result<()> {
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

pub async fn build_image_from_containerfile(name: &str) -> Result<()> {
    let client =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    let image_context_dir = {
        // HACK: only supports repo-local images
        let mut path: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        path.pop();
        path.push("images/");
        path.push(name);
        trace!("image context: {}", path.display());
        path
    };
    let tarball_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        // TODO: stream tarball
        let mut tarball = tar::Builder::new(Vec::new());
        tarball
            .append_dir_all(".", image_context_dir)
            .context("build in-memory image tarball")
            .unwrap();

        tarball
            .into_inner()
            .context("finish in-memory image tarball")
    })
    .await
    .context("spawn blocking tokio task to build tarball")??;

    let image_options = BuildImageOptions {
        dockerfile: "Containerfile",
        t: &format!("conductor/{}", name),
        labels: [
            ("io.auxon.conductor", ""),
            ("io.auxon.conductor.universe", "Foo"),
            ("io.auxon.conductor.image-context-hash", "ABCDEF01"),
        ]
        .into(),
        ..Default::default()
    };

    let mut build_image_progress =
        client.build_image(image_options, None, Some(tarball_bytes.into()));

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

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn build_image() -> Result<()> {
        const IMAGE: &str = "renode";

        build_image_from_containerfile(IMAGE).await
    }
}
