use tokio::sync::Notify;
use tokio::sync::futures::Notified;
use tokio::sync::watch;

pub struct AgentControllerPoolUpdates {
    agent_availability_change: Notify,
    update_tx: watch::Sender<()>,
}

impl AgentControllerPoolUpdates {
    pub fn agent_availability_changed(&self) -> Notified<'_> {
        self.agent_availability_change.notified()
    }

    pub fn signal(&self) {
        self.update_tx.send_replace(());
        self.agent_availability_change.notify_waiters();
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<()> {
        self.update_tx.subscribe()
    }
}

impl Default for AgentControllerPoolUpdates {
    fn default() -> Self {
        let (update_tx, _initial_rx) = watch::channel(());

        Self {
            agent_availability_change: Notify::new(),
            update_tx,
        }
    }
}
