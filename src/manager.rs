use self::fetcher::Service;
use angrepa::config;
use color_eyre::Report;
use futures::future::join_all;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

mod submitter;
use submitter::Submitters;

mod fetcher;
mod handler;

#[derive(Clone, Debug)]
pub struct Manager {
    /// raw ips
    ips: Arc<Mutex<Vec<String>>>,
    /// raw services
    services: Arc<Mutex<HashMap<String, Service>>>,
    /// last updated
    services_ips_last_tick: Arc<Mutex<Option<i32>>>,
}

impl Manager {
    pub fn new() -> Result<Self, Report> {
        Ok(Self {
            ips: Arc::new(Mutex::new(Vec::new())),
            services: Arc::new(Mutex::new(HashMap::new())),
            services_ips_last_tick: Arc::new(Mutex::new(None)),
        })
    }

    pub fn all_ips(&self) -> Vec<String> {
        self.ips.lock().clone()
    }

    /// Update ips and services
    pub fn update_ips_services(
        &self,
        tick: i32,
        ips: Vec<String>,
        services: HashMap<String, Service>,
    ) {
        let mut lock = self.ips.lock();
        *lock = ips;
        drop(lock);

        let mut lock = self.services.lock();
        *lock = services;
        drop(lock);

        let mut lock = self.services_ips_last_tick.lock();
        *lock = Some(tick);
        drop(lock);
    }

    /// Gets the ticks for this target, if it exists
    pub fn get_service_targets(&self, service_str: &str) -> Option<Service> {
        let service = {
            let lock = self.services.lock();
            lock.get(service_str).cloned()
        };

        Some(service?)
    }
}

pub async fn main(config: config::Root, manager: Manager) -> Result<(), Report> {
    let sub = Submitters::from_conf(&config.manager)?;
    let fetch = fetcher::Fetchers::from_conf(&config.manager)?;

    // run submitter on another thread
    let manager2 = manager.clone();
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
        }
    });

    // run fetcher on another thread
    let fetcher_handle = tokio::spawn(async move {
        info!("fetcher starting");

        match fetch {
            fetcher::Fetchers::Enowars(fetcher) => {
                fetcher::run(fetcher, manager2, &config.common).await
            }
            fetcher::Fetchers::Dummy(fetcher) => {
                fetcher::run(fetcher, manager2, &config.common).await
            }
        };
    });

    // join all
    join_all(vec![handler_handle, fetcher_handle]).await;

    Ok(())
}
