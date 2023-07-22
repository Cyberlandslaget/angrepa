use rocket::serde::json::{json, Value};
use rocket::{self, get, routes};

#[get("/templates")]
fn test() -> Value {
    json!({ "status": "ok" })
}

pub async fn run() {
    // TODO: Shutdown on SIGINT
    let result = rocket::build().mount("/", routes![test]).launch().await;

    result.expect("server failed unexpectedly");
}
