mod docker;
mod exploit;

use colored::Colorize;

pub enum DockerErrors {
    Build(String),
    ContainerCreate(String),
    ContainerNotRunning(String),
    ContainerStart(String),
    ContainerStop(String),
    ExecuteError(String),
    OutputParse(String),
}

pub struct OutputStd {
    stdout: String,
    stderr: String,
}

impl std::fmt::Display for OutputStd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stdout: {}\nStderr: {}", self.stdout, self.stderr.red())
    }
}

fn handle_docker_errors(e: DockerErrors) {
    match e {
        DockerErrors::Build(e) => {
            println!("{} {}", "Image build error:".red().bold(), e.red())
        }
        DockerErrors::ContainerCreate(e) => {
            println!("{} {}", "Container create error:".red().bold(), e.red())
        }
        DockerErrors::ContainerNotRunning(e) => {
            println!("{} {}", "Container running error:".red().bold(), e.red())
        }
        DockerErrors::ContainerStart(e) => {
            println!("{} {}", "Container start error:".red().bold(), e.red())
        }
        DockerErrors::ContainerStop(e) => {
            println!("{} {}", "Container stop error:".red().bold(), e.red())
        }
        DockerErrors::ExecuteError(e) => {
            println!("{} {}", "Container execute error:".red().bold(), e.red())
        }
        DockerErrors::OutputParse(e) => {
            println!("{} {}", "Stdout/err parse error:".red().bold(), e.red())
        }
    }
}

#[tokio::main]
async fn main() {
    let exp = match exploit::Exploit::init("test", 1, true).await {
        Ok(exp) => exp,
        Err(e) => return handle_docker_errors(e),
    };

    let output = match exp
        .run(
            &exp.containers[0], // TODO: Smarter way to do this, maybe som automated selector inside the run function
            "127.0.0.1".to_string(),
            "flagid_rfre".to_string(),
        )
        .await
    {
        Ok(output) => output,
        Err(e) => return handle_docker_errors(e),
    };

    println!("{output}")
}
