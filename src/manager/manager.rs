use std::collections::HashSet;

use angrapa::config;
use color_eyre::eyre::eyre;
use color_eyre::Report;
use futures::future::join_all;
use regex::Regex;

mod submitter;

mod tcp;
use tcp::Tcp;
mod web;
use web::Web;

use crate::submitter::Submitter;
use crate::submitter::{DummySubmitter, ECSCSubmitter};

#[derive(Debug)]
enum Submitters {
    Dummy(DummySubmitter),
    ECSC(ECSCSubmitter),
}

impl Submitters {
    fn from_conf(manager: config::Manager) -> Result<Self, Report> {
        match manager.submitter_name.as_str() {
            "dummy" => Ok(Self::Dummy(DummySubmitter {})),
            "ecsc" => {
                let host = manager
                    .submitter
                    .get("host")
                    .ok_or(eyre!("ECSC submitter requires host"))?;

                let host = match host {
                    toml::Value::String(s) => s.clone(),
                    _ => return Err(eyre!("ECSC submitter host must be a string")),
                };

                let header_suffix = manager
                    .submitter
                    .get("header_suffix")
                    .ok_or(eyre!("ECSC submitter requires header_suffix"))?;

                let header_suffix = match header_suffix {
                    toml::Value::String(s) => s.clone(),
                    _ => return Err(eyre!("ECSC submitter header_suffix must be a string")),
                };

                let ecsc = ECSCSubmitter::new(host, header_suffix);

                Ok(Self::ECSC(ecsc))
            }
            _ => Err(eyre!("Unknown submitter name {}", manager.submitter_name)),
        }
    }
}

async fn submitter_loop<S>(submitter: S, flag_rx: flume::Receiver<String>, flag_regex: Regex)
where
    S: Submitter + Send + Sync + 'static,
{
    // TODO chunk the submissions
    // IMPORTANT!!!!!!!!!!!!!!!!!

    let mut seen: HashSet<String> = HashSet::new();

    while let Ok(raw) = flag_rx.recv_async().await {
        let new_flags = flag_regex
            .captures_iter(&raw)
            .map(|cap| cap[0].to_string()) // take the one and only capture
            .filter(|flag| seen.insert(flag.clone()))
            .collect::<Vec<_>>();

        let r = submitter.submit(new_flags).await.unwrap();
        dbg!(&r);
    }
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;

    // get toml
    let args = argh::from_env::<angrapa::config::Args>();
    let toml = std::fs::read_to_string(args.toml)?;
    let config = toml::from_str::<angrapa::config::Root>(&toml)?;

    let flag_regex = Regex::new(&config.common.format)?;

    println!("manager");

    let sub = Submitters::from_conf(config.manager)?;

    // set up channels
    let (flag_tx, flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp_handle = {
        let flag_tx = flag_tx.clone();

        let tcp = Tcp::new("0.0.0.0:8001".to_string());
        tokio::spawn(async move {
            tcp.run(flag_tx).await.unwrap();
        })
    };

    // run web listener on another thread
    let web_handle = {
        let flag_tx = flag_tx.clone();
        let web = Web::new("0.0.0.0:8000");

        tokio::spawn(async move {
            web.run(flag_tx).await.unwrap();
        })
    };

    // run submitter on another thread
    let sub_handle = tokio::spawn(async move {
        match sub {
            Submitters::Dummy(submitter) => submitter_loop(submitter, flag_rx, flag_regex).await,
            Submitters::ECSC(submitter) => submitter_loop(submitter, flag_rx, flag_regex).await,
        }
    });

    // join all
    join_all(vec![tcp_handle, web_handle, sub_handle]).await;

    Ok(())
}
