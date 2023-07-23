use rocket::http::{ContentType, Status};
use rocket::serde::json::{json, Value};
use rocket::{self, get, routes};

#[get("/templates")]
fn list_templates() -> Value {
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
}

#[get("/templates/<template>")]
fn get_template(template: &str) -> (Status, (ContentType, Vec<u8>)) {
    let mut tar = tar::Builder::new(Vec::new());
    tar.append_dir_all(template, format!("./data/templates/{template}"))
        .unwrap();

    (Status::Ok, (ContentType::TAR, tar.into_inner().unwrap()))
}

pub async fn run() {
    // TODO: Shutdown on SIGINT
    let result = rocket::build()
        .mount("/", routes![list_templates, get_template])
        .launch()
        .await;

    result.expect("server failed unexpectedly");
}
