use futures::StreamExt;
use shiplift::rep::ContainerCreateInfo;
use shiplift::tty::TtyChunk;
use shiplift::{BuildOptions, ContainerOptions, Docker, ExecContainerOptions};

use super::{DockerErrors, OutputStd};

// TODO: config
const DATA_PATH: &str = "/home/ctf/Documents/rust/angrapa/data";

trait VecBuffBytes {
    fn to_string(self) -> Result<String, String>;
}

impl VecBuffBytes for Vec<Vec<u8>> {
    /// Flattens a `Vec<Vec<u8>>` and converts to a String
    ///
    /// # Examples
    /// ```
    /// let stdout = match stdout_vec.to_string() {
    ///     Ok(stdout) => stdout,
    ///     Err(e) => return Err(e),
    /// };
    /// ```
    fn to_string(self) -> Result<String, String> {
        use std::str::from_utf8;

        let flat = self.into_iter().flatten().collect::<Vec<u8>>();

        match from_utf8(&flat) {
            Ok(s) => Ok(s.to_owned()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }
}

/// Builds docker image and starts container for a given exploit, returns container id on success
///
/// `exploit_id`: name of the exploit directory in data/exploits
///
/// # Examples
/// ```
/// let container_id = match init_exploit("exploit_name").await {
///     Ok(id) => id,
///     Err(e) => return handle_docker_errors(e),
/// };
/// ```
pub async fn init_exploit(exploit_id: &str) -> Result<String, DockerErrors> {
    let tag = match image_build(exploit_id).await {
        Ok(tag) => tag,
        Err(e) => return Err(e),
    };

    container_create_and_start(tag).await
}

/// Builds docker image, returns the image tag on success
///
/// `exploit_id`: name of the exploit directory in data/exploits
///
/// # Examples
/// ```
/// let tag = match image_build(exploit_id).await {
///     Ok(tag) => tag,
///     Err(e) => return Err(e),
/// };
/// ```
async fn image_build(exploit_id: &str) -> Result<String, DockerErrors> {
    let tag = format!("angrapa/exploit-{exploit_id}");
    let exploit_path = format!("{DATA_PATH}/exploits/{exploit_id}");

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
                    "Failed to build image for exploit: \"{exploit_id}\"\n\t{:?}",
                    e,
                )))
            }
        }
    }

    Ok(tag)
}

/// Creates and start a docker container, returns the container id on success
///
/// `tag`: tag of the docker image
///
/// # Examples
/// ```
/// let container_id = match container_create_and_start(tag).await {
///     Ok(id) => id,
///     Err(e) => return Err(e),
/// };
/// ```
async fn container_create_and_start(tag: String) -> Result<String, DockerErrors> {
    let docker = Docker::new();

    // Create container
    let info: ContainerCreateInfo = match docker
        .containers()
        .create(&ContainerOptions::builder(&tag).build())
        .await
    {
        Ok(info) => info,
        Err(e) => {
            return Err(DockerErrors::ContainerCreate(format!(
                "Failed to create container for exploit: \"{tag}\"\n\t{:?}",
                e
            )))
        }
    };

    // Start container
    if let Err(e) = docker.containers().get(&info.id).start().await {
        return Err(DockerErrors::ContainerStart(format!(
            "Failed to start container for exploit: \"{tag}\"\n\t{:?}",
            e
        )));
    }

    Ok(info.id)
}

// TODO: exploit related stuff in seperate file
/// Execs a containers run.sh, returns stdout and stderr on success
///
/// `id`: container id
///
/// `ip`: ip of team to attack. passed as an env variable to docker
///
/// `flag_store`: flag_store, can be an empty string. passed as an env variable to docker
///
/// # Examples
/// ```
/// let output = match run_exploit(&container_id, ip, flag_id).await {
///     Ok(output) => output,
///     Err(e) => return handle_docker_errors(e),
/// };
///
/// println!("{output}");
/// ```
pub async fn run_exploit(
    id: &String,
    ip: String,
    flag_store: String,
) -> Result<OutputStd, DockerErrors> {
    // Initalize vector of env vars that are passed to the exploit
    let ip_env = format!("IP={ip}");
    let flag_store_env = format!("FLAG_STORE={flag_store}");
    let environment_vec = vec![ip_env.as_str(), flag_store_env.as_str()];

    // Initalize exec builder with entrypoint and env args, then later exec
    let docker = Docker::new();
    let options = ExecContainerOptions::builder()
        .cmd(vec!["/exploit/run.sh"])
        .env(environment_vec)
        .attach_stdout(true)
        .attach_stderr(true)
        .build();
    let mut execd = docker.containers().get(id).exec(&options);

    let mut stdout_vec: Vec<Vec<u8>> = vec![];
    let mut stderr_vec: Vec<Vec<u8>> = vec![];

    // Read the chunked output and store in vectors
    while let Some(exec_result) = execd.next().await {
        match exec_result {
            Ok(chunk) => match chunk {
                TtyChunk::StdOut(bytes) => stdout_vec.push(bytes),
                TtyChunk::StdErr(bytes) => stderr_vec.push(bytes),
                TtyChunk::StdIn(_) => unreachable!(),
            },
            Err(e) => {
                return Err(DockerErrors::ExecuteError(format!(
                    "Failed to execute container with id \"{}\"\n\t{:?}",
                    &id[..12],
                    e
                )))
            }
        }
    }

    // "Dechunk" the data and convert to strings
    let stdout = match stdout_vec.to_string() {
        Ok(stdout) => stdout,
        Err(e) => {
            return Err(DockerErrors::OutputParse(format!(
                "Failed to convert utf_8 of container exec with id \"{}\"\n\t{e}",
                &id[..12],
            )))
        }
    };

    let stderr = match stderr_vec.to_string() {
        Ok(stderr) => stderr,
        Err(e) => {
            return Err(DockerErrors::OutputParse(format!(
                "Failed to convert utf_8 of container exec with id \"{}\"\n\t{e}",
                &id[..12],
            )))
        }
    };

    Ok(OutputStd {
        stdout: String::from(stdout),
        stderr: String::from(stderr),
    })
}
