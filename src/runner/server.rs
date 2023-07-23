use rocket::{self, routes};

mod exploit;
mod templates;

use exploit::*;
use templates::*;

pub async fn run() {
    // TODO: Shutdown on SIGINT
    let result = rocket::build()
        .mount("/templates", routes![list, download])
        .mount("/exploit", routes![upload])
        .launch()
        .await;

    result.expect("server failed unexpectedly");
}
