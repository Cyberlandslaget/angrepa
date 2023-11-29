use angrepa::{config, db_connect};
use color_eyre::Report;
use futures::future::join_all;
use tracing::{info, warn};

mod submitter;
use submitter::Submitters;

mod fetcher;
mod handler;

pub async fn main(config: config::Root) -> Result<(), Report> {
    let sub = Submitters::from_conf(&config.manager)?;
    let fetch = fetcher::Fetchers::from_conf(&config)?;

    let db = db_connect(&config.database.url()).await.unwrap();

    // first insert service names
    for service in &config.common.all_services_some_renamed() {
        // a NOP if service already exists
        if let Err(e) = db.add_service_checked(service).await {
            warn!("Failed to add service: '{service}'. Error: {}", e);
        }
    }

    // run submitter on another thread
    let db_url = config.database.url();
    let handler_handle = tokio::spawn(async move {
        info!("handler starting");

        match sub {
            Submitters::Dummy(submitter) => {
                handler::run(submitter, &db_url).await;
            }
            Submitters::Faust(submitter) => {
                handler::run(submitter, &db_url).await;
            }
            Submitters::Dctf(submitter) => {
                handler::run(submitter, &db_url).await;
            }
        }
    });

    // run fetcher on another thread
    let fetcher_handle = tokio::spawn(async move {
        info!("fetcher starting");

        match fetch {
            fetcher::Fetchers::Enowars(fetcher) => fetcher::run(fetcher, &config).await,
            fetcher::Fetchers::Faust(fetcher) => fetcher::run(fetcher, &config).await,
            fetcher::Fetchers::Dummy(fetcher) => fetcher::run(fetcher, &config).await,
            fetcher::Fetchers::Statisk(fetcher) => fetcher::run(fetcher, &config).await,
        };
    });

    // join all
    join_all(vec![handler_handle, fetcher_handle]).await;

    Ok(())
}
