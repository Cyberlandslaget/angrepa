mod docker;
mod exploit;
mod exploit2;

use color_eyre::eyre;
use colored::Colorize;
use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;

const FLAG_REGEX_STR: &str = r"ECSC_[A-Za-z0-9\\+/]{32}";
const TICK_TIME: Duration = Duration::from_secs(6);
lazy_static! {
    static ref FLAG_REGEX: Regex = Regex::new(FLAG_REGEX_STR).unwrap();
}

#[derive(Clone)]
pub enum DockerErrors {
    Build(String),
    ContainerCreate(String),
    ContainerKill(String),
    ContainerStart(String),
    ExecuteError(String),
    InvalidArg(String),
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
        DockerErrors::ContainerKill(e) => {
            println!("{} {}", "Container kill error:".red().bold(), e.red())
        }
        DockerErrors::ContainerStart(e) => {
            println!("{} {}", "Container start error:".red().bold(), e.red())
        }
        DockerErrors::ExecuteError(e) => {
            println!("{} {}", "Container execute error:".red().bold(), e.red())
        }
        DockerErrors::InvalidArg(e) => {
            println!("{} {}", "Invalid argument error:".red().bold(), e.red())
        }
        DockerErrors::OutputParse(e) => {
            println!("{} {}", "Stdout/err parse error:".red().bold(), e.red())
        }
    }
}

async fn attack_example(container_count: u8) {
    let exp = match exploit::Exploit::init("test", container_count).await {
        Ok(exp) => exp,
        Err(e) => return handle_docker_errors(e),
    };

    // simulate attacking 1000 ips
    let ips = (0..1000)
        .collect::<Vec<u16>>()
        .iter()
        .map(|i| format!("172.17.0.{i}"))
        .collect::<Vec<String>>();

    // run the attack
    match exp.tick_attack(ips, "flagid_rfre".to_string()).await {
        Ok(_) => (),
        Err(e) => handle_docker_errors(e),
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    attack_example(5).await;

    Ok(())
}
