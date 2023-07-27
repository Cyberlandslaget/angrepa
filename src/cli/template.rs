use crate::{GenericResponse, Untarrer};
use argh::FromArgs;
use std::{path::PathBuf, process::exit, str::FromStr};

#[derive(FromArgs, Debug)]
/// ls or download templates
#[argh(subcommand, name = "template")]
pub struct Template {
    #[argh(subcommand)]
    /// what to do
    cmd: TemplateCommand,
}

impl Template {
    pub async fn run(&self, args: &super::Args) {
        match &self.cmd {
            TemplateCommand::Ls(x) => x.run(args).await,
            TemplateCommand::Download(x) => x.run(args).await,
        }
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
/// Template commands
pub enum TemplateCommand {
    Ls(Ls),
    Download(Download),
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "ls")]
/// List all templates
pub struct Ls {}

impl Ls {
    pub async fn run(&self, args: &super::Args) {
        let client = reqwest::Client::new();

        let url = args.host.join("/templates").unwrap();
        let resp = client.get(url).send().await.unwrap().text().await.unwrap();

        let generic: GenericResponse = serde_json::from_str(&resp).unwrap();
        if generic.status != "ok" {
            println!("Failed with error {:?}", generic.message);
            exit(1)
        }

        #[derive(serde::Deserialize, Debug)]
        struct TemplateResponse {
            templates: Vec<String>,
        }

        let template_response: TemplateResponse = serde_json::from_str(&resp).unwrap();

        for templ in template_response.templates {
            println!("- {}", templ)
        }
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "download")]
/// Download template
pub struct Download {
    #[argh(positional)]
    /// the template to download
    name: String,

    #[argh(option, default = r#"PathBuf::from_str(".").unwrap()"#)]
    /// directory to place the exploit in
    path: PathBuf,
}

impl Download {
    pub async fn run(&self, args: &super::Args) {
        let client = reqwest::Client::new();

        let endpoint = format!("/templates/{}", self.name);
        let url = args.host.join(&endpoint).unwrap();
        let resp = client.get(url).send().await.unwrap().bytes().await.unwrap();

        let templ_path = format!("templ_{}", self.name);
        let path = self.path.clone().join(templ_path);

        let untarrer = Untarrer { data: resp.into() };
        untarrer.untar(&path).unwrap();
    }
}
