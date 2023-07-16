use color_eyre::eyre;
use tokio::time::{interval_at, MissedTickBehavior};

mod exploit;
use exploit::exploit2::{
    docker::{DockerExploit, DockerExploitPool, DockerInstance},
    Exploit, ExploitInstance,
};

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
            println!("tick UTC {}", date.format("%Y-%m-%d %H:%M:%S.%f"));

            for exp in &self.exploits {
                let exp = exp.clone();
                tokio::spawn(async move {
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
                    println!("Got output {}", log.output);
                });
            }
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    // set previous time to simulate starting the program after CTF start
    let start = tokio::time::Instant::now() - tokio::time::Duration::from_secs(1000);
    let tick = tokio::time::Duration::from_secs(5);

    let mut runner = Runner {
        start,
        tick,
        exploits: vec![],
    };

    // read data/exploits/test.tar
    let tar = std::fs::read("data/exploits/new.tar")?;
    let docker = DockerInstance::new()?;

    let exploit = docker.new_exploit(tar).await?;
    let pool = exploit.spawn_pool().await?;

    runner.exploits.push(Holder::DockerPool(pool));

    runner.run().await;

    Ok(())
}
