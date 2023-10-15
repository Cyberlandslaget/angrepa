use angrepa::db::Db;
use angrepa::get_connection_pool;
use angrepa::{config, models::TargetInserter};
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Report};
use lexical_sort::natural_lexical_cmp;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use tracing::{debug, error, info, warn};

mod enowars;
pub use enowars::EnowarsFetcher;
mod dummy;
pub use dummy::DummyFetcher;
mod faust;
pub use faust::FaustFetcher;

// we have two types of APIs we need to support
//
// 1. enowars-like
// Here, the flagids of a teams' service is given as a map of (tick:int) ->
// (value), usually only giving flagids which correspond to valid flagds (not
// OLD yet)
// TLDR; GOOD: we easily know what the new flagids are
//
// 2. faust-like
// Here, the flagids of a team' service is given as an array of values. There
// is no clean & easy way to know which flagid is for what tick! The way to do
// it then, is to save previously used flagids and assume the new ones are from
// the current/new tick.
// TLDR; BAD: we dont easily know what the old flagids are

// in practice, both will return all flagids, and then our fetcher routine will
// manually remove any flagids which have been seen before

#[derive(Debug)]
pub enum Fetchers {
    Enowars(EnowarsFetcher),
    Faust(FaustFetcher),
    Dummy(DummyFetcher),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ServiceOld(pub HashMap<String, TicksOld>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TicksOld(pub HashMap<i32, serde_json::Value>);

/// All services
// /// {service_name: {"10.0.0.1": ["a", "b"], "10.0.0.2"}}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ServiceMap(HashMap<String, Service>);

impl ServiceMap {
    /// renames services
    pub fn apply_name_mapping(self, mapping: &HashMap<String, String>) -> ServiceMap {
        ServiceMap(
            self.0
                .into_iter()
                .map(|(old_name, service)| {
                    (
                        mapping.get(&old_name).unwrap_or(&old_name).to_owned(),
                        service,
                    )
                })
                .collect(),
        )
    }
}

/// A service' teams
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Service {
    teams: HashMap<String, TeamService>,
}

/// A teams' instance of a service
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TeamService {
    // in most cases there is just one flagid per tick (we always just read the
    // raw json value), but in the case of faust-like ctfs we may have multiple
    // flagids and we dont know which they belong to, so we have to put
    // multiple for the current tick
    ticks: HashMap<i32, Vec<serde_json::Value>>,
}

#[derive(thiserror::Error, Debug)]
pub enum FetcherError {
    #[error("reqwest failed")]
    Reqwest(#[from] reqwest::Error),
}

/// Implements fetching flagids and hosts
#[async_trait]
pub trait Fetcher {
    type Error: std::error::Error + Send + Sync + Debug + 'static;
    /// services (with flagids)
    async fn services(&self) -> Result<ServiceMap, Self::Error>;
    /// "backup" raw get all ips
    async fn ips(&self) -> Result<Vec<String>, Self::Error>;
}

// routine
pub async fn run(fetcher: impl Fetcher, config: &config::Root) {
    let common = &config.common;

    // 10% of 60s = 6s, a reasonable amount
    let offset = tokio::time::Duration::from_secs(common.tick) / 10;
    let mut tick_interval = common.get_tick_interval(offset).await.unwrap();

    let db_url = config.database.url();
    let db_pool = match get_connection_pool(&db_url) {
        Ok(db) => db,
        Err(e) => return warn!("Could not acquire a database pool: {e}"),
    };

    let conn = &mut match db_pool.get() {
        Ok(conn) => conn,
        Err(e) => return warn!("Could not acquire a database connection: {}", e),
    };

    let mut db = Db::new(conn);

    fetcher.ips().await.unwrap().into_iter().for_each(|ip| {
        // set default names
        let name = if Some(&ip) == config.common.nop.as_ref() {
            Some("nop")
        } else if Some(&ip) == config.common.own.as_ref() {
            Some("own")
        } else {
            None
        };

        if let Err(e) = db.add_team_checked(&ip, name) {
            warn!("Failed to add team: '{ip}'. Error: {}", e);
        }
    });

    let mut seen_flagids: HashSet<(String, String, String)> = HashSet::new();

    'outer: loop {
        // wait for new tick
        tick_interval.tick().await;
        let tick_number = common.current_tick(chrono::Utc::now());

        // get updated info
        let services = 'lp: {
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(config.common.tick / 2),
                async {
                    loop {
                        let before = std::time::Instant::now();
                        match tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            fetcher.services(),
                        )
                        .await
                        {
                            Ok(Ok(s)) => break s,
                            e => {
                                let delta = before.elapsed();
                                info!("Failed fetching {:?}: after {:?}", e, delta);
                                // wait 1s before retrying
                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                },
            )
            .await
            {
                Ok(v) => break 'lp v,
                Err(_) => {
                    warn!("Failed to fetch services, giving up for this tick");
                    continue 'outer;
                }
            };
        };

        // rename?
        let services = if let Some(ref rename) = config.common.rename {
            services.apply_name_mapping(rename)
        } else {
            services
        };

        let service_names = services.0.keys().cloned().collect::<HashSet<_>>();

        let configured_names = config.common.flagid_services_with_renames();

        if service_names != configured_names {
            let missing = service_names.difference(&configured_names);
            let extra = configured_names.difference(&service_names);

            error!(
                "Fetcher and config disagree on service names! (after applying renames) got:{:?} != fetched:{:?}",
                service_names, common.services
            );

            error!("Missing: {:?}.    Extra: {:?}", missing, extra);

            continue;
        }

        info!("tick {}", tick_number);

        let mut target_skipped = 0;
        let mut target_tried = 0;

        // services without flagid
        let all_ips = fetcher.ips().await.unwrap();

        for service_name in &common.services_without_flagid {
            for team_ip in &all_ips {
                let conn = &mut match db_pool.get() {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Could not acquire a database connection: {}", e);
                        continue;
                    }
                };

                let inserter = TargetInserter {
                    flag_id: String::from(""),
                    service: service_name.to_owned(),
                    team: team_ip.to_owned(),
                    created_at: chrono::Utc::now().naive_utc(),
                    target_tick: tick_number as i32,
                };

                let mut db = Db::new(conn);

                match db.add_target(&inserter) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Could not add target: {}", e);
                        continue;
                    }
                }
            }
        }

        // services with flagid
        for (service_name, service) in &services.0 {
            // sort by team_ip
            let mut teams = service.teams.iter().collect::<Vec<_>>();
            teams.sort_by(|a, b| natural_lexical_cmp(a.0, b.0));

            for (team_ip, service) in teams {
                for (tick, flag_ids) in &service.ticks {
                    for flag_id in flag_ids {
                        target_tried += 1;

                        let flag_id_str = match serde_json::to_string(flag_id) {
                            Ok(s) => s,
                            Err(err) => {
                                warn!("Failed to serialize flagid: {:?}", err);
                                continue;
                            }
                        };

                        // this wont work cross-restarts, but hey a few extra runs wont hurt, right? right??
                        let new = seen_flagids.insert((
                            // no tick, because ecsc, faust, etc gives all flagids, and we just remove dups
                            service_name.clone(),
                            team_ip.clone(),
                            flag_id_str,
                        ));

                        if !new {
                            target_skipped += 1;
                            continue;
                        }

                        let inserter = TargetInserter {
                            flag_id: flag_id.to_string(),
                            service: service_name.to_owned(),
                            team: team_ip.to_owned(),
                            created_at: chrono::Utc::now().naive_utc(),
                            target_tick: *tick,
                        };

                        let conn = &mut match db_pool.get() {
                            Ok(conn) => conn,
                            Err(e) => {
                                error!("Could not acquire a database connection: {}", e);
                                continue;
                            }
                        };

                        let mut db = Db::new(conn);

                        match db.add_target(&inserter) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Could not add target: {}", e);
                                continue;
                            }
                        }
                    }
                }
            }

            debug!("{} targets added, skipped {}", target_tried, target_skipped);
        }
    }
}

// Deserialize
impl Fetchers {
    pub fn from_conf(config: &config::Root) -> Result<Self, Report> {
        match config.manager.fetcher_name.as_str() {
            "dummy" => Ok(Self::Dummy(DummyFetcher {
                config: config.clone(),
            })),
            "faust" => {
                let teams = config
                    .manager
                    .fetcher
                    .get("teams")
                    .ok_or(eyre!("Faust fetcher requires teams"))?
                    .as_str()
                    .ok_or(eyre!("Faust fetcher teams must be a string"))?
                    .to_owned();

                let scoreboard = config
                    .manager
                    .fetcher
                    .get("scoreboard")
                    .ok_or(eyre!("Faust fetcher requires scoreboard"))?
                    .as_str()
                    .ok_or(eyre!("Faust fetcher scoreboard must be a string"))?
                    .to_owned();

                let format = config
                    .manager
                    .fetcher
                    .get("format")
                    .ok_or(eyre!("Faust fetcher requires format"))?
                    .as_str()
                    .ok_or(eyre!("Faust fetcher format must be a string"))?
                    .to_owned();

                Ok(Self::Faust(FaustFetcher::new(teams, scoreboard, format)))
            }
            "enowars" => {
                let endpoint = config
                    .manager
                    .fetcher
                    .get("endpoint")
                    .ok_or(eyre!("Enowars fetcher requires endpoint"))?
                    .as_str()
                    .ok_or(eyre!("Enowars fetcher endpoint must be a string"))?
                    .to_owned();

                let ips_endpoint = config
                    .manager
                    .fetcher
                    .get("ips")
                    .ok_or(eyre!("Enowars fetcher requires ip endpoint"))?
                    .as_str()
                    .ok_or(eyre!("Enowars fetcher endpoint must be a string"))?
                    .to_owned();

                Ok(Self::Enowars(EnowarsFetcher::new(endpoint, ips_endpoint)))
            }
            _ => Err(eyre!("Unknown fetcher {}", config.manager.fetcher_name)),
        }
    }
}
