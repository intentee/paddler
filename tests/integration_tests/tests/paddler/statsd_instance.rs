use tokio::process::Child;

use crate::metrics::Metrics;

#[derive(Debug, Default)]
pub struct StatsdInstance {
    pub child: Option<Child>,
    pub metrics: Vec<Metrics>,
}

impl StatsdInstance {
    pub async fn cleanup(&mut self) {
        if let Some(mut statsd) = self.child.take() {
            if let Err(err) = statsd.kill().await {
                panic!("Failed to kill statsd: {err}");
            }
        }
    }
}
