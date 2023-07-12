use futures::StreamExt;
use shiplift::rep::ContainerCreateInfo;
use shiplift::tty::TtyChunk;
use shiplift::{BuildOptions, ContainerOptions, Docker, ExecContainerOptions};
use std::str::from_utf8;

use super::{DockerErrors, OutputStd};

// TODO: config
const DATA_PATH: &str = "/home/ctf/Documents/rust/angrapa/data";

pub async fn init_exploit(exploit_id: &str) -> Result<String, DockerErrors> {
    let tag = match image_build(exploit_id).await {
        Ok(tag) => tag,
        Err(e) => return Err(e),
    };

    match container_create_and_start(tag).await {
        Ok(info) => Ok(info.id),
        Err(e) => return Err(e),
    }
}

async fn image_build(exploit_id: &str) -> Result<String, DockerErrors> {
    let docker = Docker::new();

    let tag = format!("angrapa/exploit-{exploit_id}");
    let exploit_path = format!("{DATA_PATH}/exploits/{exploit_id}");

    let options = BuildOptions::builder(exploit_path).tag(&tag).build();

    let mut stream = docker.images().build(&options);
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

async fn container_create_and_start(tag: String) -> Result<ContainerCreateInfo, DockerErrors> {
    let docker = Docker::new();

    let info = match docker
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

    let container = docker.containers().get(&info.id);

    if let Err(e) = container.start().await {
        return Err(DockerErrors::ContainerStart(format!(
            "Failed to start container for exploit: \"{tag}\"\n\t{:?}",
            e
        )));
    }

    Ok(info)
}

// TODO: exploit related stuff in seperate file
pub async fn run_exploit(
    id: &String,
    ip: String,
    flag_store: String,
) -> Result<OutputStd, DockerErrors> {
    let docker = Docker::new();

    let ip_env = ["IP", ip.as_str()].join("=");
    let flag_store_env = ["FLAG_STORE", flag_store.as_str()].join("=");

    let options = ExecContainerOptions::builder()
        .cmd(vec!["/exploit/run.sh"])
        .env(vec![ip_env.as_str(), flag_store_env.as_str()])
        .attach_stdout(true)
        .attach_stderr(true)
        .build();

    let mut execd = docker.containers().get(id).exec(&options);

    let mut stdout_vec: Vec<Vec<u8>> = vec![];
    let mut stderr_vec: Vec<Vec<u8>> = vec![];

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

    let stdout_vec = stdout_vec.into_iter().flatten().collect::<Vec<u8>>();
    let stderr_vec = stderr_vec.into_iter().flatten().collect::<Vec<u8>>();

    let stdout = match from_utf8(&stdout_vec) {
        Ok(stdout) => stdout,
        Err(e) => {
            return Err(DockerErrors::OutputParse(format!(
                "Failed to convert utf_8 of container exec with id \"{}\"\n\t{:?}",
                &id[..12],
                e
            )))
        }
    };

    let stderr = match from_utf8(&stderr_vec) {
        Ok(stderr) => stderr,
        Err(e) => {
            return Err(DockerErrors::OutputParse(format!(
                "Failed to convert utf_8 of container exec with id \"{}\"\n\t{:?}",
                &id[..12],
                e
            )))
        }
    };

    Ok(OutputStd {
        stdout: String::from(stdout),
        stderr: String::from(stderr),
    })
}
