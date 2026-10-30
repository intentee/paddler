use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::task::spawn_blocking;

pub struct OccupiedBlockingThread {
    occupant: JoinHandle<()>,
    release: Arc<Notify>,
}

impl OccupiedBlockingThread {
    #[must_use]
    pub fn occupy() -> Self {
        let release = Arc::new(Notify::new());
        let occupant_release = release.clone();
        let runtime = Handle::current();

        Self {
            occupant: spawn_blocking(move || runtime.block_on(occupant_release.notified())),
            release,
        }
    }

    pub async fn release(mut self) -> Result<(), JoinError> {
        self.release.notify_one();

        (&mut self.occupant).await
    }
}

impl Drop for OccupiedBlockingThread {
    fn drop(&mut self) {
        self.release.notify_one();
    }
}
