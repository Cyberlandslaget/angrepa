use futures::StreamExt;
use shiplift::{rep::ContainerCreateInfo, BuildOptions, ContainerOptions, Docker};

use super::DockerErrors;

// TODO: config
const DATA_PATH: &str = "/home/user/git/angrapa/data";

/// Builds docker image, returns the image tag on success
///
/// `exploit_name`: name of the exploit directory in data/exploits
///
/// # Example
/// ```
/// let tag = match image_build(exploit_name).await {
///     Ok(tag) => tag,
///     Err(e) => return Err(e),
/// };
/// ```
pub async fn image_build(exploit_name: &str) -> Result<String, DockerErrors> {
    let tag = format!("angrapa/exploit-{exploit_name}");
    let exploit_path = format!("{DATA_PATH}/exploits/{exploit_name}");

    // Initalize image builder and build the image
    let docker = Docker::new();
    let options = BuildOptions::builder(exploit_path).tag(&tag).build();
    let mut stream = docker.images().build(&options);

    // Read the output from the build process
    while let Some(build_result) = stream.next().await {
        match build_result {
            // Ok(output) => println!("{:?}", output), // TODO? print this if theres some sort of debug mode enabled?
            Ok(_) => (),
            Err(e) => {
                return Err(DockerErrors::Build(format!(
                    "Failed to build image for exploit: \"{exploit_name}\"\n\t{:?}",
                    e,
                )))
            }
        }
    }

    Ok(tag)
}

/// Creates a docker container, returns a ContainerCreateInfo struct on success
///
/// `tag: &String` tag of the docker image
///
/// # Example
/// ```
/// let info = match docker::container_create(&tag).await {
///     Ok(info) => info,
///     Err(e) => return Err(e),
/// };
/// ```
pub async fn container_create(tag: &String) -> Result<ContainerCreateInfo, DockerErrors> {
    match Docker::new()
        .containers()
        .create(&ContainerOptions::builder(tag).build())
        .await
    {
        Ok(info) => Ok(info),
        Err(e) => Err(DockerErrors::ContainerCreate(format!(
            "Failed to create container for exploit: \"{tag}\"\n\t{:?}",
            e
        ))),
    }
}
