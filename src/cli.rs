use std::process::exit;

use argh::{self, FromArgs};
use reqwest::Url;
use tokio::time::Instant;

#[derive(FromArgs, Debug)]
/// cli
struct Args {
    #[argh(
        option,
        short = 'h',
        default = r#""http://angrepa.cybl".parse().expect("invalid url (missing http://?)")"#
    )]
    /// the ataka instance to connect to
    host: Url,

    #[argh(subcommand)]
    /// what to do
    cmd: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
/// commands
enum Command {
    Ping(Ping),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "ping")]
/// ping the server
struct Ping {}

impl Ping {
    async fn run(&self, args: &Args) {
        let client = reqwest::Client::new();

        // add http:// if no prefix
        let host = if args.host.cannot_be_a_base() {
            let inner = "http://".to_owned() + args.host.as_str();
            Url::try_from(inner.as_str()).expect("failed trying to fix url")
        } else {
            args.host.clone()
        };

        let url = host.join("/ping").unwrap();

        let (resp, time) = {
            let before = Instant::now();
            let res = client.get(url.clone()).send().await;
            match res {
                Ok(res) => (res, before.elapsed()),
                Err(e) => {
                    if e.is_connect() {
                        println!("Cannot connect to '{}'!", url);
                    } else {
                        println!("Error: {:?}", e);
                    }
                    exit(1);
                }
            }
        };

        let text = resp.text().await.unwrap();
        println!("got {} in {:?}", text, time);
    }
}

#[tokio::main]
async fn main() {
    let args = argh::from_env::<Args>();
    dbg!(&args.cmd);
    match &args.cmd {
        Command::Ping(ping) => ping.run(&args).await,
    }
    println!("hello");
}
