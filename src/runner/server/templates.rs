use axum::{
    body::{Bytes, Full},
    extract::Path,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

async fn list() -> Json<Value> {
    json!(std::fs::read_dir("./data/templates")
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap()
        })
        .collect::<Vec<_>>())
    .into()
}

async fn download(Path(template): Path<String>) -> impl IntoResponse {
    let mut tar = tar::Builder::new(Vec::new());
    tar.append_dir_all(template.as_str(), format!("./data/templates/{template}"))
        .unwrap();

    let bytes = Bytes::from(tar.into_inner().unwrap());
    let body = Full::new(bytes);

    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-tar")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{template}.tar\""),
        )
        .body(body)
        .unwrap()
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(list))
        .route("/:template", get(download))
}
