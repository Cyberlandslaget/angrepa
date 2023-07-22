use color_eyre::Report;
use futures::future::join_all;
use tokio::spawn;

mod manager;
mod runner;

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrapa::config::Args>();
    let config = args.get_config()?;

    // setup logging
    args.setup_logging()?;

    let runner = runner::main(config.clone());
    let manager = manager::main(config.clone());

    let runner = spawn(async move { runner.await.unwrap() });
    let manager = spawn(async move { manager.await.unwrap() });

    join_all([runner, manager]).await;

    Ok(())
}
