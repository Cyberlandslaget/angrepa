use angrapa::{db::Db, db_connect, models::ExploitInserter};
use futures::TryStreamExt;
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr};
use tracing::{debug, info};
use warp::{multipart::FormData, reply, Buf, Filter};

use crate::runner::exploit::exploit2::docker::DockerInstance;

/// - Accepts new exploits over HTTP.
/// - Returns stats for exploits
pub struct Server {
    host: SocketAddr,
}

impl Server {
    pub fn new(host: SocketAddr) -> Self {
        Self { host }
    }

    async fn form(form: FormData) -> Result<impl warp::Reply, warp::Rejection> {
        let mut db = Db::new(angrapa::db_connect().unwrap());

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

        let json_config = if let Some(json_config) = fields.get("config") {
            json_config
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing config" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        #[derive(serde::Deserialize)]
        struct JsonConfig {
            name: String,
            service: String,
            blacklist: Vec<String>,
        }

        println!("{}", String::from_utf8_lossy(json_config));

        let json_config = match serde_json::from_slice::<JsonConfig>(json_config) {
            Ok(json_config) => json_config,
            Err(_e) => {
                info!("failed to parse json config {:?}", _e);
                return Ok(reply::with_status(
                    reply::json(&json!({ "error": format!("{:?}", _e) })),
                    warp::http::StatusCode::BAD_REQUEST,
                ));
            }
        };

        let JsonConfig {
            name,
            service,
            blacklist,
        } = json_config;

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

        let expl = db
            .add_exploit(&ExploitInserter {
                name,
                service,
                blacklist: blacklist.join("\n"),
                docker_image: pool.image,
                docker_container: pool.container,
                enabled: false,
            })
            .unwrap();

        Ok(reply::with_status(
            reply::json(&json!({ "id": expl.id })),
            warp::http::StatusCode::OK,
        ))
    }

    async fn start(id: Option<i32>) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let mut db = Db::new(angrapa::db_connect().unwrap());

        let content = match db.start_exploit(id) {
            Ok(_) => json!({ "msg": "ok" }),
            Err(err) => json!({ "error": format!("{:?}", err) }),
        };

        Ok(reply::with_status(
            reply::json(&content),
            warp::http::StatusCode::OK,
        ))
    }

    async fn stop(id: Option<i32>) -> Result<impl warp::Reply, warp::Rejection> {
        let id = if let Some(id) = id {
            id
        } else {
            return Ok(reply::with_status(
                reply::json(&json!({ "error": "missing id" })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        let mut db = Db::new(db_connect().unwrap());

        let content = match db.stop_exploit(id) {
            Ok(_) => json!({ "msg": "ok" }),
            Err(err) => json!({ "error": format!("{:?}", err) }),
        };

        Ok(reply::with_status(
            reply::json(&content),
            warp::http::StatusCode::OK,
        ))
    }

    async fn exploits() -> Result<impl warp::Reply, warp::Rejection> {
        let mut db = Db::new(db_connect().unwrap());

        let exploits = db.get_exploits().unwrap();
        let ids = exploits.iter().map(|e| e.id).collect::<Vec<_>>();

        Ok(reply::with_status(
            reply::json(&ids),
            warp::http::StatusCode::OK,
        ))
    }

    pub async fn run(&self) {
        // warp server
        let hello = warp::get().map(|| "This is the runner");

        let upload = warp::post()
            .and(warp::path("upload"))
            .and(warp::multipart::form().max_length(5_000_000))
            .map(move |form: FormData| form)
            .and_then(|form| Server::form(form));

        let start = warp::post()
            .and(warp::path("start"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| query.get("id").map(|s| s.parse().unwrap()))
            .and_then(|id: Option<i32>| Server::start(id));

        let stop = warp::post()
            .and(warp::path("stop"))
            .and(warp::query::<HashMap<String, String>>())
            .map(move |query: HashMap<String, String>| query.get("id").map(|s| s.parse().unwrap()))
            .and_then(|id: Option<i32>| Server::stop(id));

        // GET /exploits
        let exploits = warp::get()
            .and(warp::path("exploits"))
            .and_then(|| Server::exploits());

        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST"]);

        let routes = upload.or(exploits).or(start).or(stop).or(hello.with(cors));

        warp::serve(routes).run(self.host).await;
    }
}
