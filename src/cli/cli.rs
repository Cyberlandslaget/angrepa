use argh::{self, FromArgs};
use color_eyre::{eyre::eyre, Report};
use reqwest::Url;
use std::{
    io::{Cursor, Read},
    path::Path,
    process::exit,
};
use tar::Archive;
use tokio::time::Instant;

pub mod exploit;
pub mod template;

#[derive(FromArgs, Debug)]
/// cli
pub struct Args {
    #[argh(
        option,
        short = 'h',
        default = r#""http://angrepa.cybl:8000".parse().expect("invalid url")"#
    )]
    /// the ataka instance to connect to
    pub host: Url,

    #[argh(subcommand)]
    /// what to do
    cmd: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
/// commands
enum Command {
    Ping(Ping),
    Exploit(exploit::Exploit),
    Template(template::Template),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "ping")]
/// ping the server
struct Ping {}

#[tokio::main]
async fn main() {
    let mut args = argh::from_env::<Args>();

    // add http:// if no prefix
    args.host = if args.host.cannot_be_a_base() {
        let inner = "http://".to_owned() + args.host.as_str();
        Url::try_from(inner.as_str()).expect("failed trying to fix url")
    } else {
        args.host
    };

    match &args.cmd {
        Command::Ping(ping) => ping.run(&args).await,
        Command::Exploit(expl) => expl.run(&args).await,
        Command::Template(templ) => templ.run(&args).await,
    }
}

impl Ping {
    async fn run(&self, args: &Args) {
        let client = reqwest::Client::new();

        let url = args.host.join("/ping").unwrap();

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

fn tarify(path: &str) -> Result<Vec<u8>, Report> {
    use tar::Builder;

    let mut tar = Builder::new(Vec::new());

    tar.append_dir_all(".", path)?;
    tar.finish()?;

    Ok(tar.into_inner()?)
}

pub struct Untarrer {
    pub data: Vec<u8>,
}

impl Untarrer {
    pub fn untar(self, out_dir: &Path) -> Result<(), Report> {
        let mut tar = Archive::new(Cursor::new(self.data));
        std::fs::create_dir_all(out_dir).unwrap();

        for file in tar.entries()? {
            let mut file = file?;

            let full_path = out_dir.clone().join(file.header().path()?);

            if full_path
                .to_str()
                .ok_or(eyre!("failed to stringify path"))?
                .ends_with('/')
            {
                std::fs::create_dir_all(full_path)?
            } else {
                let mut data = Vec::new();
                file.read_to_end(&mut data)?;
                println!("{}", full_path.display());

                std::fs::write(full_path, data)?;
            }
        }

        Ok(())
    }
}

#[derive(serde::Deserialize, Debug)]
struct GenericResponse {
    status: String,
    message: Option<String>,
}

impl GenericResponse {
    fn success(&self) -> bool {
        self.status == "ok"
    }
}
