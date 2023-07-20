use color_eyre::Report;
use futures::future::join_all;
use regex::Regex;
use tracing::info;

mod submitter;
use submitter::Submitters;

mod listener;
use listener::{Tcp, Web};

mod handler;

mod fetcher;

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get config
    let args = argh::from_env::<angrapa::config::Args>();
    let config = args.get_config()?;

    // setup logging
    args.setup_logging()?;

    let flag_regex = Regex::new(&config.common.format)?;

    info!("manager started");

    let sub = Submitters::from_conf(&config.manager)?;

    // set up channels
    let (raw_flag_tx, raw_flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp_listener = {
        let flag_tx = raw_flag_tx.clone();

        let host = config.manager.tcp_listener.parse()?;
        let tcp = Tcp::new(host);

        info!("tcp listener starting on {}:{}", host.ip(), host.port());

        tokio::spawn(async move {
            tcp.run(flag_tx).await.unwrap();
        })
    };

    // run web listener on another thread
    let http_listener = {
        let flag_tx = raw_flag_tx.clone();

        let host = config.manager.http_listener.parse()?;
        let web = Web::new(host);

        info!("http listener starting on {}:{}", host.ip(), host.port());

        tokio::spawn(async move {
            web.run(flag_tx).await.unwrap();
        })
    };

    // run submitter on another thread
    let handler_handle = tokio::spawn(async move {
        info!("handler starting");

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
    join_all(vec![tcp_listener, http_listener, handler_handle]).await;

    Ok(())
}
