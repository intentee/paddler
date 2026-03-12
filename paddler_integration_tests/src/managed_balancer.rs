use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use paddler_client::PaddlerClient;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::process::Child;
use tokio::process::Command;
use url::Url;

use crate::PADDLER_BINARY_PATH;
use crate::POLL_INTERVAL;
use crate::TIMEOUT;

pub struct ManagedBalancerParams {
    pub buffered_request_timeout: Duration,
    pub compat_openai_addr: Option<String>,
    pub inference_addr: String,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Option<Duration>,
    pub management_addr: String,
    pub management_cors_allowed_hosts: Vec<String>,
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

        let mut command = Command::new(PADDLER_BINARY_PATH);

        command
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
            .stderr(std::process::Stdio::piped());

        if let Some(openai_addr) = &params.compat_openai_addr {
            command.arg("--compat-openai-addr").arg(openai_addr);
        }

        if let Some(inference_item_timeout) = &params.inference_item_timeout {
            command
                .arg("--inference-item-timeout")
                .arg(inference_item_timeout.as_millis().to_string());
        }

        for host in &params.inference_cors_allowed_hosts {
            command.arg("--inference-cors-allowed-host").arg(host);
        }

        for host in &params.management_cors_allowed_hosts {
            command.arg("--management-cors-allowed-host").arg(host);
        }

        let child = command.spawn()?;

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
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await
                && snapshot.agents.len() == expected
            {
                return snapshot.agents.len();
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {expected} agents");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_desired_state(&self, expected_state: &BalancerDesiredState) {
        let start = std::time::Instant::now();

        loop {
            if let Ok(state) = self.client.management().get_balancer_desired_state().await
                && &state == expected_state
            {
                return;
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for desired state to be applied");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_buffered_requests(&self, expected: i32) -> i32 {
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_buffered_requests().await
                && snapshot.buffered_requests_current == expected
            {
                return snapshot.buffered_requests_current;
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {expected} buffered requests");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_total_desired_slots(&self, expected_total: i32) -> i32 {
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await {
                let total: i32 = snapshot
                    .agents
                    .iter()
                    .map(|agent| agent.desired_slots_total)
                    .sum();

                if total >= expected_total {
                    return total;
                }
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {expected_total} total desired slots");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_total_slots(&self, expected_total: i32) -> i32 {
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await {
                let total: i32 = snapshot.agents.iter().map(|agent| agent.slots_total).sum();

                if total >= expected_total {
                    return total;
                }
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {expected_total} total slots");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_slots_processing(&self, expected_total: i32) -> i32 {
        let start = std::time::Instant::now();

        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await {
                let total: i32 = snapshot
                    .agents
                    .iter()
                    .map(|agent| agent.slots_processing)
                    .sum();

                if total >= expected_total {
                    return total;
                }
            }

            if start.elapsed() > TIMEOUT {
                panic!("timed out waiting for {expected_total} slots processing");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn wait_for_agent_issue(
        &self,
        predicate: impl Fn(&AgentIssue) -> bool,
    ) -> AgentIssue {
        loop {
            if let Ok(snapshot) = self.client.management().get_agents().await {
                for agent in &snapshot.agents {
                    if let Some(issue) = agent.issues.iter().find(|issue| predicate(issue)) {
                        return issue.clone();
                    }
                }
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.child.kill().await?;

        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.start_kill()?;

        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
                Err(_) => break,
            }
        }

        Ok(())
    }

    async fn wait_until_ready(&self) -> Result<()> {
        let start = std::time::Instant::now();

        loop {
            let response = self.client.management().get_agents().await;

            if response.is_ok() {
                return Ok(());
            }

            if start.elapsed() > TIMEOUT {
                bail!("Balancer did not become ready within {TIMEOUT:?}");
            }

            tokio::time::sleep(POLL_INTERVAL).await;
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
