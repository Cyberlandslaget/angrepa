use futures::TryStreamExt;
use std::{collections::HashMap, net::SocketAddr};
use tracing::{debug, info};
use warp::{multipart::FormData, reply, Buf, Filter};

use crate::{AttackTarget, DockerInstance, ExploitHolder, Exploits};

/// - Accepts new exploits over HTTP.
/// - Returns stats for exploits
pub struct Server {
    host: SocketAddr,
    exploit_tx: flume::Sender<ExploitHolder>,
}

impl Server {
    pub fn new(host: SocketAddr, exploit_tx: flume::Sender<ExploitHolder>) -> Self {
        Self { host, exploit_tx }
    }

    async fn form(
        form: FormData,
        exploit_tx: flume::Sender<ExploitHolder>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let fields = form
            .and_then(|mut field| async move {
                let mut bytes: Vec<u8> = Vec::new();

                while let Some(content) = field.data().await {
                    let content = content.unwrap();
                    let chunk = content.chunk();
                    bytes.extend_from_slice(chunk);
                }

                let key = field.name().to_string();
                let value = bytes;

                Ok((key, value))
            })
            .try_collect::<HashMap<_, _>>()
            .await
            .unwrap();

        let tar = if let Some(tar) = fields.get("tar") {
            tar
        } else {
            return Ok(reply::with_status(
                "missing tar",
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        debug!("got tar of length {}", tar.len());

        // spawn a task to build the exploit
        let docker = DockerInstance::new().unwrap();
        let exploit = docker.new_exploit(tar).await;

        let exploit = match exploit {
            Ok(exploit) => exploit,
            Err(_e) => {
                return Ok(reply::with_status(
                    "error building exploit",
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        let pool = exploit.spawn_pool().await.unwrap();

        let exp = ExploitHolder {
            enabled: false,
            // TODO, actually select target
            target: AttackTarget::Ips,
            exploit: Exploits::DockerPool(pool),
        };

        info!("Successfully build new exploit");
        exploit_tx.send_async(exp).await.unwrap();

        Ok(reply::with_status("ok", warp::http::StatusCode::OK))
    }

    pub async fn run(&self) {
        // warp server
        let hello = warp::get().map(|| "This is the runner");

        let exploit_tx = self.exploit_tx.clone();
        let upload = warp::post()
            .and(warp::path("upload"))
            .and(warp::multipart::form().max_length(5_000_000))
            .map(move |form: FormData| {
                let tx = exploit_tx.clone();
                (form, tx)
            })
            .and_then(|(f, tx)| Server::form(f, tx));

        let routes = hello.or(upload);
        warp::serve(routes).run(self.host).await;
    }
}
