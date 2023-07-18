use color_eyre::Report;
use futures::future::join_all;
use lazy_static::lazy_static;
use regex::Regex;

mod submitter;

mod tcp;
use tcp::Tcp;
mod web;
use web::Web;

use crate::submitter::Submitter;

const FLAG_REGEX_STR: &str = r"ECSC_[A-Za-z0-9\\+/]{32}";
lazy_static! {
    static ref FLAG_REGEX: Regex = Regex::new(FLAG_REGEX_STR).unwrap();
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;
    println!("manager");

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
        let submitter = submitter::DummySubmitter {};

        while let Ok(raw) = flag_rx.recv_async().await {
            // extract the flags
            let mut flags = Vec::new();

            for cap in FLAG_REGEX.captures_iter(&raw) {
                flags.push(cap[0].to_string());
            }

            let r = submitter.submit(flags).await.unwrap();
            dbg!(&r);
        }
    });

    // join all
    join_all(vec![tcp_handle, web_handle, sub_handle]).await;

    Ok(())
}
