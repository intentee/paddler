use dashmap::DashMap;
use tokio::process::Child;

#[derive(Debug, Default)]
pub struct AgentsCollection {
    pub monitoring_interval: Option<i64>,
    pub instances: DashMap<String, Child>,
}

impl AgentsCollection {
    pub async fn cleanup(&mut self) {
        for mut agent in self.instances.iter_mut() {
            if let Err(err) = agent.value_mut().kill().await {
                panic!("Failed to kill agent {}: {}", agent.key(), err);
            }
        }

        self.instances.clear();
    }
}
