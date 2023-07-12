use colored::Colorize;

mod docker;

pub enum DockerErrors {
    Build(String),
    ContainerCreate(String),
    ContainerStart(String),
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
            return println!("{} {}", "Image build error:".red().bold(), e.red())
        }
        DockerErrors::ContainerCreate(e) => {
            return println!("{} {}", "Container create error:".red().bold(), e.red())
        }
        DockerErrors::ContainerStart(e) => {
            return println!("{} {}", "Container start error:".red().bold(), e.red())
        }
        DockerErrors::ExecuteError(e) => {
            return println!("{} {}", "Container execute error:".red().bold(), e.red())
        }
        DockerErrors::OutputParse(e) => {
            return println!("{} {}", "Stdout/err parse error:".red().bold(), e.red())
        }
    }
}

#[tokio::main]
async fn main() {
    let container_id = match docker::init_exploit("test").await {
        Ok(id) => id,
        Err(e) => return handle_docker_errors(e),
    };

    let output = match docker::run_exploit(
        &container_id,
        "127.0.0.1".to_string(),
        "flagid_rfre".to_string(),
    )
    .await
    {
        Ok(output) => output,
        Err(e) => return handle_docker_errors(e),
    };

    println!("{output}");
}
