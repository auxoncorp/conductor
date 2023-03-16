use anyhow::{Context, Result};
use bollard::{
    container,
    container::RemoveContainerOptions,
    exec::{CreateExecOptions, StartExecResults},
    image::CreateImageOptions,
    Docker,
};
use futures_util::{StreamExt, TryStreamExt};
use std::default::Default;
use tracing::{error, trace};

pub async fn run_in_docker(image: impl AsRef<str>, command: Vec<&str>) -> Result<()> {
    let docker =
        Docker::connect_with_local_defaults().context("connect to container system service")?;

    let image_response = docker
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
    let container = docker
        .create_container::<&str, &str>(None, container_config)
        .await?;

    trace!(?container, "created container");

    docker
        .start_container::<String>(&container.id, None)
        .await?;

    trace!("started container");

    let exec = docker
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
    } = docker.start_exec(&exec.id, None).await?
    {
        trace!("container exec started");
        while let Some(Ok(msg)) = output.next().await {
            print!("{msg}");
        }
    } else {
        error!("failed to attach to container");
        panic!();
    }

    docker
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

        let docker = Docker::connect_with_local_defaults()?;

        let images = docker.list_images::<&str>(None).await?;
        trace!("local images: {images:#?}");

        let image = docker.inspect_image("ubuntu1").await;
        trace!("ubuntu image: {image:#?}");

        let search_options = SearchImagesOptions {
            term: "docker.io/rust",
            ..Default::default()
        };
        let searched_images = docker.search_images(search_options).await;
        trace!("searched images: {searched_images:#?}");

        Ok(())
    }

    #[tokio::test]
    #[traced_test]
    async fn hello_world() -> Result<()> {
        const IMAGE: &str = "docker.io/ubuntu:latest";

        run_in_docker(IMAGE, vec!["uname", "-a"]).await
    }
}
