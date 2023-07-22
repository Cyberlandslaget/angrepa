use rocket::{self, get, routes};

#[get("/")]
fn test() -> &'static str {
    "Hello, world!"
}

pub async fn run() {
    // TODO: Shutdown on SIGINT
    let result = rocket::build().mount("/", routes![test]).launch().await;

    result.expect("server failed unexpectedly");
}
