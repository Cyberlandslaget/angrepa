use color_eyre::Report;
use std::net::SocketAddr;
use warp::{hyper::body::Bytes, Filter};

pub struct Web {
    host: SocketAddr,
}

impl Web {
    pub fn new(host: &str) -> Self {
        let host = host.parse().unwrap();
        Self { host }
    }

    pub async fn run(&self, tx: flume::Sender<String>) -> Result<(), Report> {
        let hello = warp::get().map(|| "Hello, World!");

        let post = warp::post()
            .and(warp::path("submit"))
            .and(warp::body::bytes())
            .map(move |body: Bytes| {
                let tx = tx.clone();
                (tx, body)
            })
            .and_then(|(tx, body): (flume::Sender<String>, Bytes)| async move {
                let body = String::from_utf8(body.into()).unwrap();
                tx.send_async(body).await.unwrap();
                Ok::<_, warp::Rejection>("ok")
            });

        let routes = hello.or(post);

        warp::serve(routes).run(self.host).await;

        Ok(())
    }
}
