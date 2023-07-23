use angrepa::{db::Db, db_connect, models::FlagModel};
use tokio::spawn;
use tracing::info;

use super::submitter::{FlagStatus, Submitter};

/// Submits flags
async fn submit(
    mut db: Db,
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flags: Vec<FlagModel>,
) {
    let flag_strings = flags.iter().map(|f| f.text.clone()).collect::<Vec<_>>();

    let results = submitter.submit(flag_strings).await.unwrap();
    for flag in flags {
        db.set_flag_submitted(flag.id).unwrap();
    }

    let accepted = results
        .iter()
        .filter(|(_, status)| matches!(status, FlagStatus::Accepted));

    info!(
        "Got {} results, {} accepted.",
        results.len(),
        accepted.count()
    );

    for (flag_str, status) in results {
        db.update_flag_status(&flag_str, &status.to_string())
            .unwrap();
    }
}

pub async fn run(submitter: impl Submitter + Send + Sync + Clone + 'static) {
    // submit every 5s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(5));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let mut db = Db::new(db_connect().unwrap());
        send_signal.tick().await;

        // extract out flags from the queue, then delete them
        let unsubmitted = db.get_unsubmitted_flags().unwrap();

        spawn(submit(db, submitter.clone(), unsubmitted));
    }
}
