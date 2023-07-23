mod templates;

use rocket::{self, routes};
use templates::*;

pub async fn run() {
    // TODO: Shutdown on SIGINT
    let result = rocket::build()
        .mount("/templates", routes![list, download])
        .launch()
        .await;

    result.expect("server failed unexpectedly");
}
