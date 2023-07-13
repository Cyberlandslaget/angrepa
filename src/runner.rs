mod docker;
mod exploit_futures;
mod exploit_pool;
mod myrunner;
mod dockerrunner;

use color_eyre::eyre;
use colored::Colorize;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::time::Instant;

const FLAG_REGEX_STR: &str = r"ECSC_[A-Za-z0-9\\+/]{32}";
lazy_static! {
    static ref FLAG_REGEX: Regex = Regex::new(FLAG_REGEX_STR).unwrap();
}

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

impl OutputStd {
    /// Function to extract all flags from stdout and stderr
    ///
    /// # Example
    /// ```
    /// println!("{}", output.flags().join("\n"));
    /// ```
    pub fn flags(&self) -> Vec<String> {
        let mut flags: Vec<String> = vec![];

        for cap in FLAG_REGEX.captures_iter(&self.stdout) {
            flags.push(cap[0].to_string());
        }

        for cap in FLAG_REGEX.captures_iter(&self.stderr) {
            flags.push(cap[0].to_string());
        }

        flags
    }
}

impl std::fmt::Display for OutputStd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stdout: {}\nStderr: {}", self.stdout, self.stderr.red())
    }
}

pub fn handle_docker_errors(e: DockerErrors) {
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

async fn pool_main() {
    let exp = match exploit_pool::Exploit::init("test", 10).await {
        Ok(exp) => exp,
        Err(e) => return handle_docker_errors(e),
    };
}

async fn futures_main() {
    let exp = match exploit_futures::Exploit::init("test").await {
        Ok(exp) => exp,
        Err(e) => return handle_docker_errors(e),
    };

    let mut tasks = Vec::new();
    for i in 0..100 {
        let local_exp = exp.clone();
        tasks.push(tokio::spawn(async move {
            let now = Instant::now();
            let output = match local_exp
                .run(format!("172.17.0.{}", i), "flagid_rfre".to_string())
                .await
            {
                Ok(output) => output,
                Err(_e) => return println!("error"),
            };
        }));
    }

    futures::future::join_all(tasks).await;
    println!("All done!");
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    if true {
        futures_main().await;
    } else {
        pool_main().await;
    }

    Ok(())
}
