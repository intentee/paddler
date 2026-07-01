use std::time::SystemTime;

use tokio::process::Child;

use crate::balancer_management_client::BalancerManagementClient;

#[derive(Debug, Default)]
pub struct BalancerInstance {
    pub allowed_cors_hosts: Vec<String>,
    pub buffered_request_timeout: Option<i64>,
    pub child: Option<Child>,
    pub last_update: Option<SystemTime>,
    pub management_client: BalancerManagementClient,
    pub max_buffered_requests: Option<i64>,
    pub statsd_reporting_interval: Option<i64>,
}

impl BalancerInstance {
    pub async fn cleanup(&mut self) {
        if let Some(mut balancer) = self.child.take() {
            if let Err(err) = balancer.kill().await {
                panic!("Failed to kill balancer: {err}");
            }
        }

        self.allowed_cors_hosts.clear();
    }
}
