use std::process::exit;

use argh::FromArgs;

use crate::GenericResponse;

#[derive(FromArgs, Debug)]
/// ls or download templates
#[argh(subcommand, name = "template")]
pub struct Template {
    #[argh(subcommand)]
    /// what to do
    cmd: TemplateCommands,
}

impl Template {
    pub async fn run(&self, args: &super::Args) {
        match &self.cmd {
            TemplateCommands::Ls(ls) => ls.run(args).await,
        }
    }
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
/// Template commands
pub enum TemplateCommands {
    Ls(Ls),
}

#[derive(FromArgs, Debug)]
/// List all templates
#[argh(subcommand, name = "ls")]
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
