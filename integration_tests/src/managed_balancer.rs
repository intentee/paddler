use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use paddler_client::PaddlerClient;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::process::Child;
use tokio::process::Command;
use url::Url;

use crate::managed_agent::PADDLER_BINARY_PATH;

pub struct ManagedBalancerParams {
    pub buffered_request_timeout: Duration,
    pub inference_addr: String,
    pub management_addr: String,
    pub max_buffered_requests: i32,
    pub state_database_path: String,
}

pub struct ManagedBalancer {
    child: Child,
    client: PaddlerClient,
}

impl ManagedBalancer {
    pub async fn spawn(params: ManagedBalancerParams) -> Result<Self> {
        let state_database_url = format!("file://{}", params.state_database_path);

        let child = Command::new(PADDLER_BINARY_PATH)
            .arg("balancer")
            .arg("--inference-addr")
            .arg(&params.inference_addr)
            .arg("--management-addr")
            .arg(&params.management_addr)
            .arg("--state-database")
            .arg(&state_database_url)
            .arg("--max-buffered-requests")
            .arg(params.max_buffered_requests.to_string())
            .arg("--buffered-request-timeout")
            .arg(params.buffered_request_timeout.as_millis().to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let inference_url = Url::parse(&format!("http://{}", params.inference_addr))?;
        let management_url = Url::parse(&format!("http://{}", params.management_addr))?;
        let client = PaddlerClient::new(inference_url, management_url, 1);

        let managed_balancer = Self { child, client };

        managed_balancer.wait_until_ready().await?;

        Ok(managed_balancer)
    }

    pub fn client(&self) -> &PaddlerClient {
        &self.client
    }

    pub async fn wait_for_agent_count(&self, expected: usize) -> usize {
        let timeout = Duration::from_secs(3);
        let poll_interval = Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await
                && snapshot.agents.len() == expected
            {
                return snapshot.agents.len();
            }

            if start.elapsed() > timeout {
                panic!("timed out waiting for {expected} agents");
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    pub async fn wait_for_desired_state(&self, expected_state: &BalancerDesiredState) {
        let timeout = Duration::from_secs(3);
        let poll_interval = Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            if let Ok(state) = self.client.management().get_balancer_desired_state().await
                && &state == expected_state
            {
                return;
            }

            if start.elapsed() > timeout {
                panic!("timed out waiting for desired state to be applied");
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.start_kill()?;

        Ok(())
    }

    async fn wait_until_ready(&self) -> Result<()> {
        let timeout = Duration::from_secs(5);
        let poll_interval = Duration::from_millis(50);
        let start = std::time::Instant::now();

        loop {
            let response = self.client.management().get_agents().await;

            if response.is_ok() {
                return Ok(());
            }

            if start.elapsed() > timeout {
                bail!("Balancer did not become ready within {timeout:?}");
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

impl Drop for ManagedBalancer {
    fn drop(&mut self) {
        if let Err(error) = self.kill() {
            eprintln!("Failed to kill managed balancer: {error}");
        }
    }
}
