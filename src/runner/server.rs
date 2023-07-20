use futures::TryStreamExt;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr};
use tracing::{debug, info};
use warp::{multipart::FormData, reply, Buf, Filter};

use crate::{AttackTarget, DockerInstance, ExploitHolder, Exploits, Runner};

/// - Accepts new exploits over HTTP.
/// - Returns stats for exploits
pub struct Server {
    host: SocketAddr,
    runner: Runner,
}

impl Server {
    pub fn new(host: SocketAddr, runner: Runner) -> Self {
        Self { host, runner }
    }

    async fn form(form: FormData, mut runner: Runner) -> Result<impl warp::Reply, warp::Rejection> {
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
        runner.register_exp(exp).await;

        Ok(reply::with_status(
            reply::json(&json!({ "id": id })),
            warp::http::StatusCode::OK,
        ))
    }

    async fn start(
        id: Option<String>,
        mut runner: Runner,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let content = match runner.start(&id).await {
            Ok(_) => json!({ "msg": "ok" }),
            Err(err) => json!({ "error": format!("{:?}", err) }),
        };

        Ok(reply::with_status(
            reply::json(&content),
            warp::http::StatusCode::OK,
        ))
    }

    async fn stop(
        id: Option<String>,
        mut runner: Runner,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let content = match runner.stop(&id).await {
            Ok(_) => json!({ "msg": "ok" }),
            Err(err) => json!({ "error": format!("{:?}", err) }),
        };

        Ok(reply::with_status(
            reply::json(&content),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn run(&self) {
        // warp server
        let hello = warp::get().map(|| "This is the runner");

        let rnr = self.runner.clone();
        let upload = warp::post()
            .and(warp::path("upload"))
            .and(warp::multipart::form().max_length(5_000_000))
            .map(move |form: FormData| {
                let rnr = rnr.clone();
                (form, rnr)
            })
            .and_then(|(f, rnr)| Server::form(f, rnr));

        let rnr = self.runner.clone();
        let start = warp::post()
            .and(warp::path("start"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let rnr = rnr.clone();
                (query.get("id").map(|s| s.to_string()), rnr)
            })
            .and_then(|(id, rnr)| Server::start(id, rnr));

        let rnr = self.runner.clone();
        let stop = warp::post()
            .and(warp::path("stop"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let rnr = rnr.clone();
                (query.get("id").map(|s| s.to_string()), rnr)
            })
            .and_then(|(id, rnr)| Server::stop(id, rnr));

        let routes = hello.or(upload).or(start).or(stop);
        warp::serve(routes).run(self.host).await;
    }
}
