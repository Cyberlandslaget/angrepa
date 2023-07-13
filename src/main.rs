mod docker;
mod exploit;

use crate::exploit::Exploit;

use colored::Colorize;
use std::sync::Arc;
use tokio::task::JoinSet;

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

// Simulate attacking 100 teams using all (10) containers
async fn tick_attack_test(exp: &mut Arc<exploit::Exploit>, flag_store: String) {
    let ips = (1..=100)
        .map(|n| format!("10.0.{n}.2"))
        .collect::<Vec<String>>();

    let containers = exp.running_containers();

    let mut set = JoinSet::new();

    (0..containers.len()).for_each(|t: usize| {
        let start = t * ips.len() / containers.len();
        let end = (t + 1) * ips.len() / containers.len();

        let task_ips = ips[start..end].to_vec();

        set.spawn({
            let exp = Arc::clone(&exp);
            let flag_store = flag_store.clone();

            async move {
                for ip in task_ips {
                    let _output = match exp.containers[t].run(ip.to_string(), &flag_store).await {
                        Ok(output) => output,
                        Err(e) => return handle_docker_errors(e),
                    };

                    // println!("{output}");
                }
            }
        });
    });

    while let Some(_) = set.join_next().await {}
}

#[tokio::main]
async fn main() {
    let exp = match Exploit::init("test", 10, true).await {
        Ok(exp) => exp,
        Err(e) => return handle_docker_errors(e),
    };

    let exp = &mut Arc::new(exp);
    // Simulate 10 attacks (10 ticks)
    for _ in 0..10 {
        tick_attack_test(exp, String::from("flag_store_test")).await;
    }
}
