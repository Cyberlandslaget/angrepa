use color_eyre::Report;
use futures::future::join_all;
use tokio::spawn;

mod manager;
mod runner;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref ARGS: angrepa::config::Args = argh::from_env::<angrepa::config::Args>();
    pub static ref CONFIG: angrepa::config::Root = ARGS.get_config().unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // setup logging
    ARGS.setup_logging()?;

    // make these here so they are the same across
    let manager = manager::Manager::new()?;

    let runner_thr = runner::main(manager.clone());
    let manager_thr = manager::main(manager);

    let runner = spawn(async move { runner_thr.await.unwrap() });
    let manager = spawn(async move { manager_thr.await.unwrap() });

    join_all([runner, manager]).await;

    Ok(())
}
