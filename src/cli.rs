use color_eyre::Report;
use serde_json::json;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
    process::exit,
};
use tar::Archive;

use argh::{self, FromArgs};
use reqwest::{
    multipart::{Form, Part},
    Url,
};
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
    Upload(Upload),
    Download(Download),
    Start(Start),
    Stop(Stop),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "ping")]
/// ping the server
struct Ping {}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "upload")]
/// upload an exploit
struct Upload {
    #[argh(positional)]
    /// path to the exploit
    exploit: PathBuf,

    #[argh(option)]
    /// name of service
    service: Option<String>,

    #[argh(option)]
    /// name of exploit
    name: Option<String>,

    #[argh(option)]
    /// pool size
    pool: Option<usize>,

    #[argh(option)]
    /// blacklist, ex. "10.0.0.1, 10.0.0.2"
    blacklist: Option<String>,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "download")]
/// download an exploit
struct Download {
    #[argh(positional)]
    /// exploit id
    id: i32,

    #[argh(option)]
    /// directory to place the exploit folder in
    path: PathBuf,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "start")]
/// start an exploit
struct Start {
    #[argh(positional)]
    /// id of exploit
    id: i32,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "stop")]
/// stop an exploit
struct Stop {
    #[argh(positional)]
    /// id of exploit
    id: i32,
}

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
        Command::Upload(upload) => upload.run(&args).await,
        Command::Download(download) => download.run(&args).await,
        Command::Start(start) => start.run(&args).await,
        Command::Stop(stop) => stop.run(&args).await,
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

impl Upload {
    async fn run(&self, args: &Args) {
        // make sure all arguments are valid
        std::fs::read_dir(&self.exploit).expect("failed to read exploit directory");
        let name = self.name.as_ref().expect("missing name");
        let service = self.service.as_ref().expect("missing service");
        let blacklist: Vec<_> = self
            .blacklist
            .as_ref()
            .map(|text| text.split(',').map(|ip| ip.trim()).collect())
            .unwrap_or(vec![]);
        let pool = self.pool;

        // build tar
        let tar = tarify(self.exploit.to_str().unwrap()).expect("failed to tar exploit");

        println!("Uploading {}B file", tar.len());

        // upload
        let client = reqwest::Client::new();
        let url = args.host.join("/exploit/upload").unwrap();

        let config = json!({
            "service": service,
            "name": name,
            "blacklist": blacklist,
            "pool": pool,
        });

        let form = Form::new()
            .text("config", config.to_string())
            .part("tar", Part::bytes(tar).file_name("exploit.tar"));

        let resp = client
            .post(url.clone())
            .multipart(form)
            .send()
            .await
            .unwrap();

        #[derive(serde::Deserialize)]
        struct BuildResponse {
            id: i32,
            status: String,
        }

        let build = resp.json::<BuildResponse>().await.unwrap();

        if build.status == "ok" {
            println!("Sucessfully built exploit {}", build.id);
        } else {
            println!("Failed to build: {}", build.status);
        }
    }
}

impl Download {
    async fn run(&self, args: &Args) {
        // make sure all arguments are valid
        std::fs::read_dir(&self.path).expect("failed to read output directory. does it exist?");
        let dirname = format!("download_{}", self.id);
        let out_dir = self.path.join(dirname);
        println!("{}", out_dir.to_str().unwrap());
        std::fs::create_dir_all(&out_dir).unwrap();

        // download
        let client = reqwest::Client::new();
        let endpoint = format!("/exploit/download/{}", self.id);
        let url = args.host.join(&endpoint).unwrap();

        let tar = client.get(url).send().await.unwrap().bytes().await.unwrap();

        // untar
        let mut tar = Archive::new(Cursor::new(tar));
        for file in tar.entries().unwrap() {
            let mut file = file.unwrap();

            let full_path = out_dir.clone().join(file.header().path().unwrap());

            if full_path.to_str().unwrap().ends_with("/") {
                std::fs::create_dir_all(full_path).unwrap()
            } else {
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                println!("{}", full_path.display());

                std::fs::write(full_path, data).unwrap();
            }
        }
    }
}

#[derive(serde::Deserialize, Debug)]
struct GenericResponse {
    status: String,
    msg: Option<String>,
}

impl GenericResponse {
    fn is_ok(&self) -> bool {
        self.status == "ok"
    }
}

impl Start {
    async fn run(&self, args: &Args) {
        let client = reqwest::Client::new();

        let endpoint = format!("/exploit/start/{}", self.id);
        let url = args.host.join(&endpoint).unwrap();

        let resp: GenericResponse = client
            .post(url.clone())
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        if resp.is_ok() {
            println!("Started exploit {}", self.id);
        } else {
            println!(
                "Failed to start exploit {}: {}",
                self.id,
                resp.msg.unwrap_or_default()
            );
        }
    }
}

impl Stop {
    async fn run(&self, args: &Args) {
        let client = reqwest::Client::new();

        let endpoint = format!("/exploit/stop/{}", self.id);
        let url = args.host.join(&endpoint).unwrap();

        let resp: GenericResponse = client
            .post(url.clone())
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        if resp.is_ok() {
            println!("Stopped exploit {}", self.id);
        } else {
            println!(
                "Failed to stop exploit {}: {}",
                self.id,
                resp.msg.unwrap_or_default()
            );
        }
    }
}
