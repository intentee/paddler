use tokio::sync::watch;

pub trait SubscribesToUpdates {
    fn subscribe_to_updates(&self) -> watch::Receiver<()>;
}
