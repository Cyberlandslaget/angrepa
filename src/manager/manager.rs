use color_eyre::Report;
use futures::future::join_all;
use warp::Filter;

mod submitter;

mod tcp;
use tcp::Tcp;

struct Manager {}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;
    println!("manager");

    // run tcp listener on another thread
    let tcp = Tcp::new("0.0.0.0:8001".to_string());
    let tcp_handle = tokio::spawn(async move {
        tcp.run().await.unwrap();
    });

    // run web listener on another thread
    let routes = warp::any().map(|| "Hello, World!");
    let web_handle = tokio::spawn(async move {
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    });

    // join all
    join_all(vec![tcp_handle, web_handle]).await;

    Ok(())
}
