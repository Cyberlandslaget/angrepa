use std::collections::BTreeMap;
use tracing::{field::Visit, Level, Subscriber};
use tracing_subscriber::Layer;
use webhook::client::WebhookClient;

pub struct WebhookLayer {
    event_tx: flume::Sender<String>,
}

impl WebhookLayer {
    pub fn new(url: String) -> Self {
        let client = WebhookClient::new(&url);

        let (event_tx, event_rx) = flume::unbounded::<String>();

        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv_async().await {
                loop {
                    let result = client.send(|message| message.content(&event)).await;
                    if let Err(err) = result {
                        // if it is a ratelimit, then logging the error will make it worse...
                        eprintln!("failed to send webhook: {:?}", err);

                        // is this golang or something?
                        if format!("{:?}", err).contains("rate limited") {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    } else {
                        // success
                        break;
                    }
                }
            }
        });

        Self { event_tx }
    }
}

impl<S> Layer<S> for WebhookLayer
where
    S: Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // warn or worse
        if event.metadata().level() > &Level::WARN {
            return;
        }

        let mut fields = BTreeMap::new();
        let mut visitor = Visitor(&mut fields);
        event.record(&mut visitor);

        let unix_timestamp = chrono::Utc::now().timestamp();

        let message = fields
            .get("message")
            .cloned()
            .unwrap_or("no message".to_string());

        let note = if fields.len() > 1 {
            format!(" *({} other fields not shown)*", fields.len() - 1)
        } else {
            "".to_string()
        };

        let msg = format!(
            "<t:{timestamp}:T> **{level}** `{target}`: {message}{note}",
            timestamp = unix_timestamp,
            level = event.metadata().level(),
            target = event.metadata().target(),
            message = message,
            note = note
        );

        self.event_tx.send(msg).unwrap();
    }
}

struct Visitor<'a>(&'a mut BTreeMap<String, String>);
impl<'a> Visit for Visitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_owned());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{:?}", value));
    }

    // TODO: the rest
}
