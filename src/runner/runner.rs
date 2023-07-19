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
    start: tokio::time::Instant,
    tick: tokio::time::Duration,
    exploits: Vec<Holder>,
}

impl Runner {
    async fn run(&self) {
        let mut interval = interval_at(self.start, self.tick);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            // print clock
            let date = chrono::Utc::now();
            info!("tick UTC {}", date.format("%Y-%m-%d %H:%M:%S.%f"));

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
    info!("Manager started!");

    let time_since_start = chrono::Utc::now() - common.start;

    let start = tokio::time::Instant::now() - time_since_start.to_std()?
        // start 1 sec into the tick just in case
        + tokio::time::Duration::from_secs(1);

    info!("CTF started {:?} ago", time_since_start);

    let tick = tokio::time::Duration::from_secs(common.tick);

    let mut runner = Runner {
        start,
        tick,
        exploits: vec![],
    };

    let tar = tarify("data/exploits/new")?;
    let docker = DockerInstance::new()?;

    let exploit = docker.new_exploit(tar).await?;
    let pool = exploit.spawn_pool().await?;

    runner.exploits.push(Holder::DockerPool(pool));
    runner.exploits.push(Holder::Docker(exploit));

    runner.run().await;

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
