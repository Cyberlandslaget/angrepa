use angrapa::config::Common;
use color_eyre::eyre;
use tokio::time::{interval_at, MissedTickBehavior};

mod exploit;
use exploit::exploit2::{
    docker::{DockerExploit, DockerExploitPool, DockerInstance},
    Exploit, ExploitInstance,
};
use tracing::info;

#[derive(Clone)]
enum Holder {
    DockerPool(DockerExploitPool),
    Docker(DockerExploit),
}

struct Runner {
    exploits: Vec<Holder>,
}

impl Runner {
    async fn run(&self, conf: &Common) {
        let mut interval = conf
            // make sure the tick has started
            .get_tick_interval(tokio::time::Duration::from_secs(1))
            .await
            .unwrap();

        loop {
            interval.tick().await;

            // print clock
            let date = chrono::Utc::now();
            let current_tick = conf.current_tick(date);
            info!(
                "tick {} (UTC {})",
                current_tick,
                date.format("%Y-%m-%d %H:%M:%S.%f")
            );

            for exp in &self.exploits {
                let exp = exp.clone();
                tokio::spawn(async move {
                    let before = tokio::time::Instant::now();
                    let log = match exp {
                        Holder::DockerPool(pool) => {
                            let inst = pool
                                .start("1.2.3.4".to_string(), "fakeid".to_string())
                                .await
                                .unwrap();
                            inst.wait_for_exit().await.unwrap()
                        }
                        Holder::Docker(single) => {
                            let inst = single
                                .start("1.2.3.4".to_string(), "fakeid".to_string())
                                .await
                                .unwrap();
                            inst.wait_for_exit().await.unwrap()
                        }
                    };
                    let elapsed = before.elapsed();
                    info!("Execution took {:?}, output: {:?}", elapsed, log.output)
                });
            }
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrapa::config::Args>();
    let toml = args.get_config()?;
    let common = toml.common;

    // setup logging
    args.setup_logging()?;

    // time until start
    common.sleep_until_start().await;
    assert!(chrono::Utc::now() >= common.start);
    info!("Manager woke up!");

    let time_since_start = chrono::Utc::now() - common.start;

    info!("CTF started {:?} ago", time_since_start);

    let tick = tokio::time::Duration::from_secs(common.tick);

    let mut runner = Runner { exploits: vec![] };

    let tar = tarify("data/exploits/new")?;
    let docker = DockerInstance::new()?;

    let exploit = docker.new_exploit(tar).await?;
    let pool = exploit.spawn_pool().await?;

    runner.exploits.push(Holder::DockerPool(pool));
    runner.exploits.push(Holder::Docker(exploit));

    runner.run(&common).await;

    Ok(())
}

fn tarify(path: &str) -> eyre::Result<Vec<u8>> {
    use tar::Builder;

    let mut tar = Builder::new(Vec::new());

    tar.append_dir_all(".", path)?;
    tar.finish()?;

    let tar = tar.into_inner()?;
    Ok(tar)
}
