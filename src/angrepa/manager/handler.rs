use angrepa::{db::Db, db_connect};
use std::collections::HashSet;
use tokio::spawn;
use tracing::{info, trace};

use super::submitter::{FlagStatus, Submitter};

/// Submits flags
async fn submit(
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flag_strings: Vec<String>,
    db: Db,
) {
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
            .await
            .unwrap();
    }
}

pub async fn run(submitter: impl Submitter + Send + Sync + Clone + 'static, db_url: &str) {
    let db = db_connect(db_url).await.unwrap();

    // submit every 3s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(3));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut seen_flags: HashSet<String> = HashSet::new();

    loop {
        send_signal.tick().await;

        // extract out flags from the queue, then delete them
        let flags = db.get_unsubmitted_flags().await.unwrap();
        let mut flag_strings: Vec<String> = Vec::new();

        for flag in &flag_strings {
            seen_flags.insert(flag.clone());
        }

        // preemtively mark them as submitted
        for flag in flags {
            if seen_flags.contains(&flag.text) {
                continue;
            }

            db.set_flag_submitted(flag.id).await.unwrap();
            flag_strings.push(flag.text);
        }

        let chunks = flag_strings.chunks(150);
        trace!("chunk len {}", chunks.len());
        for flags in chunks {
            let db = db.clone();
            spawn(submit(submitter.clone(), flags.to_vec(), db));
        }
    }
}
