use futures::TryStreamExt;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr};
use tracing::{debug, info};
use warp::{multipart::FormData, reply, Buf, Filter};

use crate::{AttackTarget, DockerInstance, ExploitHolder, Exploits, RunnerRequest};

/// - Accepts new exploits over HTTP.
/// - Returns stats for exploits
pub struct Server {
    host: SocketAddr,
    exploit_tx: flume::Sender<ExploitHolder>,
    request_tx: flume::Sender<RunnerRequest>,
}

impl Server {
    pub fn new(
        host: SocketAddr,
        exploit_tx: flume::Sender<ExploitHolder>,
        request_tx: flume::Sender<RunnerRequest>,
    ) -> Self {
        Self {
            host,
            exploit_tx,
            request_tx,
        }
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
                reply::json(&json!({ "error": "missing tar" })),
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
                    reply::json(&json!({ "error": format!("{:?}", _e) })),
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        let pool = exploit.spawn_pool().await.unwrap();

        let id = format!("{:x}", rand::random::<u64>());

        let exp = ExploitHolder {
            id: id.clone(),
            enabled: false,
            // TODO, actually select target
            target: AttackTarget::Ips,
            exploit: Exploits::DockerPool(pool),
        };

        info!("Successfully build new exploit");
        exploit_tx.send_async(exp).await.unwrap();

        Ok(reply::with_status(
            reply::json(&json!({ "id": id })),
            warp::http::StatusCode::OK,
        ))
    }

    async fn start(
        id: Option<String>,
        request_tx: flume::Sender<RunnerRequest>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        request_tx
            .send_async(RunnerRequest::Start(id.to_string()))
            .await
            .unwrap();

        Ok(reply::with_status(
            reply::json(&json!({ "msg": "ok" })),
            warp::http::StatusCode::OK,
        ))
    }

    async fn stop(
        id: Option<String>,
        request_tx: flume::Sender<RunnerRequest>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        request_tx
            .send_async(RunnerRequest::Stop(id.to_string()))
            .await
            .unwrap();

        Ok(reply::with_status(
            reply::json(&json!({ "msg": "ok" })),
            warp::http::StatusCode::OK,
        ))
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

        let rtx = self.request_tx.clone();
        let start = warp::post()
            .and(warp::path("start"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let tx = rtx.clone();
                (query.get("id").map(|s| s.to_string()), tx)
            })
            .and_then(|(id, tx)| Server::start(id, tx));

        let rtx = self.request_tx.clone();
        let stop = warp::post()
            .and(warp::path("stop"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let tx = rtx.clone();
                (query.get("id").map(|s| s.to_string()), tx)
            })
            .and_then(|(id, tx)| Server::stop(id, tx));

        let routes = hello.or(upload).or(start).or(stop);
        warp::serve(routes).run(self.host).await;
    }
}
