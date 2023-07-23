use axum::{http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};

// #[get("/")]
// pub fn list() -> Value {
//     json!(std::fs::read_dir("./data/templates")
//         .unwrap()
//         .filter_map(|e| e.ok())
//         .map(|e| {
//             e.path()
//                 .file_name()
//                 .unwrap()
//                 .to_os_string()
//                 .into_string()
//                 .unwrap()
//         })
//         .collect::<Vec<_>>())
// }

// #[get("/<template>")]
// pub fn download(template: &str) -> (Status, (ContentType, Vec<u8>)) {
//     let mut tar = tar::Builder::new(Vec::new());
//     tar.append_dir_all(template, format!("./data/templates/{template}"))
//         .unwrap();

//     (Status::Ok, (ContentType::TAR, tar.into_inner().unwrap()))
// }

async fn root() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, json!({ "status": "ok" }).into())
}

pub fn router() -> Router {
    Router::new().route("/", get(root))
}
