use color_eyre::Report;
use futures::future::join_all;
use regex::Regex;

mod submitter;
use submitter::Submitters;

mod listener;
use listener::{Tcp, Web};

mod handler;

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get toml
    let args = argh::from_env::<angrapa::config::Args>();
    let toml = std::fs::read_to_string(args.toml)?;
    let config = toml::from_str::<angrapa::config::Root>(&toml)?;

    let flag_regex = Regex::new(&config.common.format)?;

    println!("manager");

    let sub = Submitters::from_conf(&config.manager)?;

    // set up channels
    let (raw_flag_tx, raw_flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp_handle = {
        let flag_tx = raw_flag_tx.clone();

        let tcp = Tcp::new(config.manager.tcp_listener.parse()?);
        tokio::spawn(async move {
            tcp.run(flag_tx).await.unwrap();
        })
    };

    // run web listener on another thread
    let web_handle = {
        let flag_tx = raw_flag_tx.clone();
        let web = Web::new(config.manager.http_listener.parse()?);

        tokio::spawn(async move {
            web.run(flag_tx).await.unwrap();
        })
    };

    // run submitter on another thread
    let sub_handle = tokio::spawn(async move {
        match sub {
            Submitters::Dummy(submitter) => {
                handler::run(raw_flag_rx, submitter, flag_regex).await;
            }
            Submitters::Faust(submitter) => {
                handler::run(raw_flag_rx, submitter, flag_regex).await;
            }
        }
    });

    // join all
    join_all(vec![tcp_handle, web_handle, sub_handle]).await;

    Ok(())
}
