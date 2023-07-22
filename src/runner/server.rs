use futures::TryStreamExt;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr};
use tracing::{debug, info};
use warp::{multipart::FormData, reply, Buf, Filter};

use super::{AttackTarget, DockerInstance, ExploitHolder, Exploits, Runner};

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
            run_logs: HashMap::new(),
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

    async fn logs_ticks(
        id: Option<String>,
        runner: Runner,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let mut ticks = {
            let lock = runner.exploits.lock();
            let instance = lock.get(&id);
            let instance = match instance {
                Some(instance) => instance,
                None => {
                    return Ok(reply::with_status(
                        reply::json(&json!({ "error": "no such exploit" })),
                        warp::http::StatusCode::BAD_REQUEST,
                    ));
                }
            };
            instance.run_logs.keys().cloned().collect::<Vec<_>>()
        };
        ticks.sort();

        let content = json!({ "ticks": ticks });

        Ok(reply::with_status(
            reply::json(&content),
            warp::http::StatusCode::OK,
        ))
    }

    async fn log(
        id: Option<String>,
        tick: Option<i64>,
        runner: Runner,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let tick = if let Some(tick) = tick {
            tick
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing tick" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let log = {
            let lock = runner.exploits.lock();

            let instance = lock.get(&id);
            let instance = match instance {
                Some(instance) => instance,
                None => {
                    return Ok(reply::with_status(
                        reply::json(&json!({ "error": "no such id" })),
                        warp::http::StatusCode::BAD_REQUEST,
                    ));
                }
            };

            let log = instance.run_logs.get(&tick);
            let log = match log {
                Some(log) => log,
                None => {
                    return Ok(reply::with_status(
                        reply::json(&json!({ "error": "no such tick" })),
                        warp::http::StatusCode::BAD_REQUEST,
                    ));
                }
            };

            log.clone()
        };

        Ok(reply::with_status(
            reply::json(&json!({ "log": log.log.output })),
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

        // GET /log/ticks?id=abc
        // -> returns all ticks that have logs
        let rnr = self.runner.clone();
        let log_ticks = warp::get()
            .and(warp::path("log"))
            .and(warp::path("ticks"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let rnr = rnr.clone();
                (query.get("id").map(|s| s.to_string()), rnr)
            })
            .and_then(|(id, rnr)| Server::logs_ticks(id, rnr));

        // GET /log?id=abc&tick=123
        let rnr = self.runner.clone();
        let log = warp::get()
            .and(warp::path("log"))
            .and(warp::path("get"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| {
                let rnr = rnr.clone();
                let id = query.get("id").map(|s| s.to_string());
                let tick: Option<i64> = query.get("tick").and_then(|s| s.parse().ok());
                (id, tick, rnr)
            })
            .and_then(|(id, tick, rnr)| Server::log(id, tick, rnr));

        // GET /exploits
        let rnr = self.runner.clone();
        let exploits = warp::get()
            .and(warp::path("exploits"))
            .map(move || {
                let rnr = rnr.clone();
                rnr
            })
            .and_then(|rnr: Runner| async move {
                let lock = rnr.exploits.lock();
                let ids = lock.keys().cloned().collect::<Vec<_>>();
                Ok::<_, warp::Rejection>(reply::json(&ids))
            });

        let routes = log
            .or(log_ticks)
            .or(upload)
            .or(exploits)
            .or(start)
            .or(stop)
            .or(hello);

        // disable cors
        let cors = warp::cors().allow_any_origin();

        warp::serve(routes.with(cors)).run(self.host).await;
    }
}
