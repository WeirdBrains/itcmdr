use once_cell::sync::Lazy;
use tokio::sync::broadcast;
use crate::generated::itcmdr::Update;

static BUS: Lazy<broadcast::Sender<Update>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(1024);
    tx
});

pub fn init() {
    let _ = BUS.clone();
}

pub fn subscribe() -> broadcast::Receiver<Update> {
    BUS.subscribe()
}

pub fn notify(kind: &str, entity: &str, id: &str) {
    let update = Update {
        kind: kind.to_string(),
        entity: entity.to_string(),
        id: id.to_string(),
        ts: chrono::Utc::now().timestamp_millis(),
    };
    let _ = BUS.send(update);
}

