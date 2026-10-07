use actix_ws::Session;
use tokio::time::Duration;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

const PING_INTERVAL: Duration = Duration::from_secs(3);

pub struct SessionKeepAlive {
    pub connection_close: CancellationToken,
    pub session: Session,
}

impl SessionKeepAlive {
    pub async fn run(mut self) {
        let mut ping_ticker = interval(PING_INTERVAL);

        ping_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        while self
            .connection_close
            .run_until_cancelled(ping_ticker.tick())
            .await
            .is_some()
            && self.session.ping(b"").await.is_ok()
        {}

        self.connection_close.cancel();
    }
}
