use color_eyre::eyre::Report;
use tokio::io::AsyncReadExt;
use warp::Filter;

mod submitter;

struct Manager {}

#[tokio::main]
async fn main() -> Result<(), Report> {
    color_eyre::install()?;
    println!("manager");

    tokio::spawn(async move {
        tcp_listener().await.unwrap();
    })
    .await?;

    let routes = warp::any().map(|| "Hello, World!");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

async fn tcp_listener() -> Result<(), Report> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            // read everything for 30s, then timeout if it is not closed
            let read_text =
                tokio::time::timeout(std::time::Duration::from_secs(30), read_all(socket)).await;

            dbg!(String::from_utf8(read_text.unwrap().unwrap()));
        });
    }
}

async fn read_all(mut socket: tokio::net::TcpStream) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = [0; 1024];
    let mut read_text = Vec::new();
    loop {
        match socket.read(&mut buf).await {
            Ok(n) => {
                if n == 0 {
                    break;
                }
                read_text.extend(&buf[..n]);
            }
            Err(e) => {
                return Result::Err(e);
            }
        }
    }
    Ok(read_text)
}
