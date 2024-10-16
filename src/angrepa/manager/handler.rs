use angrepa::{db::Db, db_connect, get_connection_pool};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use std::collections::HashSet;
use tokio::spawn;
use tracing::{info, trace};

use super::submitter::{FlagStatus, Submitter};

/// Submits flags
async fn submit(
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flag_strings: Vec<String>,
    mut conn: PooledConnection<ConnectionManager<PgConnection>>,
) {
    let mut db = Db::new(&mut conn);

    let results = submitter.submit(flag_strings).await.unwrap();

    let accepted = results
        .iter()
        .filter(|(_, status)| matches!(status, FlagStatus::Ok));

    if !results.is_empty() {
        info!(
            "Got {} results, {} accepted.",
            results.len(),
            accepted.count()
        );
    }

    for (flag_str, status) in results {
        db.update_flag_status(&flag_str, &status.to_string())
            .unwrap();
    }
}

pub async fn run(submitter: impl Submitter + Send + Sync + Clone + 'static, db_url: &str) {
    let mut conn = db_connect(db_url).unwrap();
    let mut db = Db::new(&mut conn);

    // submit every 3s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(3));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut seen_flags: HashSet<String> = HashSet::new();
    let db_pool = get_connection_pool(db_url).unwrap();

    loop {
        send_signal.tick().await;

        // extract out flags from the queue, then delete them
        let flags = db.get_unsubmitted_flags().unwrap();
        let mut flag_strings: Vec<String> = Vec::new();

        for flag in &flag_strings {
            seen_flags.insert(flag.clone());
        }

        // preemtively mark them as submitted
        for flag in flags {
            if seen_flags.contains(&flag.text) {
                continue;
            }

            db.set_flag_submitted(flag.id).unwrap();
            flag_strings.push(flag.text);
        }

        let chunks = flag_strings.chunks(150);
        trace!("chunk len {}", chunks.len());
        for flags in chunks {
            let conn = db_pool.get().unwrap();
            spawn(submit(submitter.clone(), flags.to_vec(), conn));
        }
    }
}
