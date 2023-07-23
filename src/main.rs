use color_eyre::Report;
use futures::future::join_all;
use tokio::spawn;

mod manager;
mod runner;

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrepa::config::Args>();
    let config = args.get_config()?;

    // setup logging
    args.setup_logging()?;

    // make these here so they are the same across
    let manager = manager::Manager::new()?;

    let runner_thr = runner::main(config.clone(), manager.clone());
    let manager_thr = manager::main(config.clone(), manager);

    let runner = spawn(async move { runner_thr.await.unwrap() });
    let manager = spawn(async move { manager_thr.await.unwrap() });

    join_all([runner, manager]).await;

    Ok(())
}
