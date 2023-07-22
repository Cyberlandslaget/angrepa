use tokio::{select, spawn};
use tracing::{debug, info};

use super::{
    submitter::{FlagStatus, Submitter},
    Manager,
};

/// Submits flags
async fn submit(
    manager: Manager,
    submitter: impl Submitter + Send + Sync + Clone + 'static,
    flags: Vec<String>,
) {
    info!("Submitting {:?}", flags);
    let results = submitter.submit(flags).await.unwrap();

    let accepted = results
        .iter()
        .filter(|(_, status)| matches!(status, FlagStatus::Accepted));

    info!(
        "Got {} results, {} accepted.",
        results.len(),
        accepted.count()
    );

    for (flag_str, status) in results {
        debug!("Flag {} is {:?}", flag_str, status);
        manager.update_flag_status(&flag_str, status);
    }
}

pub async fn run(manager: Manager, submitter: impl Submitter + Send + Sync + Clone + 'static) {
    // submit every 5s
    let mut send_signal = tokio::time::interval(std::time::Duration::from_secs(5));
    send_signal.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let manager = manager.clone();
        send_signal.tick().await;

        // extract out flags from the queue, then delete them
        let mut lock = manager.flag_queue.lock();
        let to_submit = lock.drain(..).collect::<Vec<_>>();
        drop(lock);

        // get the raw text
        let to_submit = to_submit.iter().map(|f| f.flag.clone()).collect::<Vec<_>>();

        spawn(submit(manager, submitter.clone(), to_submit));
    }
}
