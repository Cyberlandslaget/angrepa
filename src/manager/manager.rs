use std::str::FromStr;

use angrapa::schema::flags::dsl::flags;
use angrapa::{db_connect, models::FlagModel};
use color_eyre::Report;
use diesel::RunQueryDsl;
use futures::future::join_all;
use regex::Regex;
use tracing::{error, info};

mod submitter;
use submitter::{FlagStatus, Submitters};

mod listener;
use listener::{Tcp, Web};

mod handler;

mod fetcher;

pub struct Flag {
    pub flag: String,
    pub tick: Option<i32>,
    pub stamp: Option<chrono::NaiveDateTime>,
    pub exploit_id: Option<String>,
    pub target_ip: Option<String>,
    pub flagstore: Option<String>,
    pub sent: bool,
    pub status: Option<FlagStatus>,
}

impl Flag {
    pub fn from_model(model: FlagModel) -> Self {
        let FlagModel {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        } = model;

        let status = if let Some(status_str) = status {
            let status = FlagStatus::from_str(&status_str);
            match status {
                Ok(status) => Some(status),
                Err(e) => {
                    error!("Error parsing status: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        }
    }

    pub fn to_model(self) -> FlagModel {
        let Flag {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        } = self;

        let status = status.map(|s| s.to_string());

        FlagModel {
            flag,
            tick,
            stamp,
            exploit_id,
            target_ip,
            flagstore,
            sent,
            status,
        }
    }
}

//pub struct Manager {
//    // mutex
//    flags: HashMap<String, Flag>,
//}

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

    // check flags in db
    let db = &mut db_connect()?;
    let existing_flags: Vec<FlagModel> = flags.load(db)?;
    info!("found {} flags in db", existing_flags.len());

    let sub = Submitters::from_conf(&config.manager)?;
    let fetch = fetcher::Fetchers::from_conf(&config.manager)?;

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

    // run fetcher on another thread
    let fetcher_handle = tokio::spawn(async move {
        info!("fetcher starting");

        match fetch {
            fetcher::Fetchers::Enowars(fetcher) => fetcher::run(fetcher, &config.common).await,
            fetcher::Fetchers::Dummy(fetcher) => fetcher::run(fetcher, &config.common).await,
        };
    });

    // join all
    join_all(vec![
        tcp_listener,
        http_listener,
        handler_handle,
        fetcher_handle,
    ])
    .await;

    Ok(())
}
