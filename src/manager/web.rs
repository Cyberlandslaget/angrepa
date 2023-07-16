use color_eyre::Report;
use std::net::SocketAddr;
use warp::Filter;

pub struct Web {
    host: SocketAddr,
}

impl Web {
    pub fn new(host: &str) -> Self {
        let host = host.parse().unwrap();
        Self { host }
    }

    pub async fn run(&self) -> Result<(), Report> {
        let routes = warp::any().map(|| "Hello, World!");
        warp::serve(routes).run(self.host).await;
        Ok(())
    }
}
