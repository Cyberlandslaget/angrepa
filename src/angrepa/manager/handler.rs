use angrepa::{db::Db, db_connect};
use tokio::spawn;
use tracing::info;

use super::submitter::{FlagStatus, Submitter};

/// Submits flags
async fn submit(submitter: impl Submitter + Send + Sync + Clone + 'static, db_url: String) {
    let mut conn = db_connect(&db_url).unwrap();
    let mut db = Db::new(&mut conn);

    // extract out flags from the queue, then delete them
    let flags = db.get_unsubmitted_flags().unwrap();

    let flag_strings = flags.iter().map(|f| f.text.clone()).collect::<Vec<_>>();

    let results = submitter.submit(flag_strings).await.unwrap();
    for flag in flags {
        db.set_flag_submitted(flag.id).unwrap();
    }

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
    // submit every 1s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(1));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        send_signal.tick().await;
        spawn(submit(submitter.clone(), db_url.to_owned()));
    }
}
