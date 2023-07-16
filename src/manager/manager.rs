use std::net::SocketAddr;

use color_eyre::Report;
use futures::future::join_all;
use warp::Filter;

mod submitter;

mod tcp;
use tcp::Tcp;
mod web;
use web::Web;

struct Manager {}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;
    println!("manager");

    // set up channels
    let (flag_tx, flag_rx) = flume::unbounded::<String>();

    // run tcp listener on another thread
    let tcp = Tcp::new("0.0.0.0:8001".to_string());
    let tcp_handle = tokio::spawn(async move {
        tcp.run(flag_tx).await.unwrap();
    });

    // run web listener on another thread
    let web = Web::new("127.0.0.1:3030");
    let web_handle = tokio::spawn(async move {
        web.run().await.unwrap();
    });

    // run submitter on another thread
    let sub_handle = tokio::spawn(async move {
        while let Ok(flag) = flag_rx.recv_async().await {
            let flag = flag.trim();
            println!("Received flags: {}", flag);
        }
    });

    // join all
    join_all(vec![tcp_handle, web_handle, sub_handle]).await;

    Ok(())
}
