use std::time::Duration;

use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_client::PaddlerClient;
use paddler_types::agent_desired_state::AgentDesiredState;
use paddler_types::agent_issue::AgentIssue;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::process::Child;
use tokio::time::timeout;
use url::Url;

use crate::BALANCER_READY_TIMEOUT;
use crate::WAIT_FOR_EVENT_IDLE_TIMEOUT;
use crate::managed_balancer_params::ManagedBalancerParams;
use crate::paddler_command;
use crate::terminate_child;
use crate::wait_for_stream_predicate::wait_for_stream_predicate;

const HEALTH_PROBE_BACKOFF: Duration = Duration::from_millis(50);

pub struct ManagedBalancer {
    child: Child,
    client: PaddlerClient,
    compat_openai_addr: String,
    inference_addr: String,
    management_addr: String,
}

impl ManagedBalancer {
    pub async fn spawn(params: ManagedBalancerParams) -> Result<Self> {
        let mut command = paddler_command();

        command
            .arg("balancer")
            .arg("--inference-addr")
            .arg(&params.inference_addr)
            .arg("--management-addr")
            .arg(&params.management_addr)
            .arg("--state-database")
            .arg(&params.state_database_url)
            .arg("--max-buffered-requests")
            .arg(params.max_buffered_requests.to_string())
            .arg("--buffered-request-timeout")
            .arg(params.buffered_request_timeout.as_millis().to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        command
            .arg("--compat-openai-addr")
            .arg(&params.compat_openai_addr);

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
        let compat_openai_url = Url::parse(&format!("http://{}", params.compat_openai_addr))?;
        let client = PaddlerClient::new(inference_url.clone(), management_url.clone(), 1);

        wait_until_ready(&management_url, &inference_url, &compat_openai_url).await?;

        Ok(Self {
            child,
            client,
            compat_openai_addr: params.compat_openai_addr,
            inference_addr: params.inference_addr,
            management_addr: params.management_addr,
        })
    }

    #[must_use]
    pub const fn client(&self) -> &PaddlerClient {
        &self.client
    }

    #[must_use]
    pub fn inference_addr(&self) -> &str {
        &self.inference_addr
    }

    #[must_use]
    pub fn management_addr(&self) -> &str {
        &self.management_addr
    }

    #[must_use]
    pub fn compat_openai_addr(&self) -> &str {
        &self.compat_openai_addr
    }

    pub async fn wait_for_agent_count(&self, expected: usize) -> Result<usize> {
        let stream = self.client.management().agents_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                if snapshot.agents.len() == expected {
                    Some(snapshot.agents.len())
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "expected agent count",
        )
        .await
    }

    pub async fn wait_for_desired_state(
        &self,
        expected_state: &BalancerDesiredState,
    ) -> Result<()> {
        let stream = self
            .client
            .management()
            .balancer_desired_state_stream()
            .await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |state| {
                if state == expected_state {
                    Some(())
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "balancer desired state",
        )
        .await
    }

    pub async fn wait_for_applicable_state(
        &self,
        expected_state: &AgentDesiredState,
    ) -> Result<()> {
        let stream = self
            .client
            .management()
            .balancer_applicable_state_stream()
            .await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |state| {
                if state.as_ref() == Some(expected_state) {
                    Some(())
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "balancer applicable state",
        )
        .await
    }

    pub async fn wait_for_buffered_requests(&self, expected: i32) -> Result<i32> {
        let stream = self.client.management().buffered_requests_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                if snapshot.buffered_requests_current == expected {
                    Some(snapshot.buffered_requests_current)
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "buffered request count",
        )
        .await
    }

    pub async fn wait_for_total_desired_slots(&self, expected_total: i32) -> Result<i32> {
        let stream = self.client.management().agents_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                let total: i32 = snapshot
                    .agents
                    .iter()
                    .map(|agent| agent.desired_slots_total)
                    .sum();

                if total >= expected_total {
                    Some(total)
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "total desired slots",
        )
        .await
    }

    pub async fn wait_for_total_slots(&self, expected_total: i32) -> Result<i32> {
        let stream = self.client.management().agents_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                let total: i32 = snapshot.agents.iter().map(|agent| agent.slots_total).sum();

                if total >= expected_total {
                    Some(total)
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "total slots",
        )
        .await
    }

    pub async fn wait_for_slots_processing(&self, expected_total: i32) -> Result<i32> {
        let stream = self.client.management().agents_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                let total: i32 = snapshot
                    .agents
                    .iter()
                    .map(|agent| agent.slots_processing)
                    .sum();

                if total >= expected_total {
                    Some(total)
                } else {
                    None
                }
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "processing slots",
        )
        .await
    }

    pub async fn wait_for_agent_issue<TPredicate>(
        &self,
        predicate: TPredicate,
    ) -> Result<AgentIssue>
    where
        TPredicate: Fn(&AgentIssue) -> bool + Send + Sync,
    {
        let stream = self.client.management().agents_stream().await?;

        wait_for_stream_predicate(
            stream.map(|result| result.map_err(anyhow::Error::from)),
            |snapshot| {
                snapshot
                    .agents
                    .iter()
                    .flat_map(|agent| agent.issues.iter())
                    .find(|issue| predicate(issue))
                    .cloned()
            },
            WAIT_FOR_EVENT_IDLE_TIMEOUT,
            "matching agent issue",
        )
        .await
    }

    pub fn shutdown(&mut self) -> Result<()> {
        terminate_child(&mut self.child);

        Ok(())
    }

    pub fn kill(&mut self) {
        terminate_child(&mut self.child);
    }

    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    pub async fn wait_for_exit(&mut self) -> Result<std::process::ExitStatus> {
        Ok(self.child.wait().await?)
    }
}

impl Drop for ManagedBalancer {
    fn drop(&mut self) {
        self.kill();
    }
}

async fn wait_until_ready(
    management_url: &Url,
    inference_url: &Url,
    compat_openai_url: &Url,
) -> Result<()> {
    let health_client = reqwest::Client::new();

    let probe = async {
        tokio::try_join!(
            wait_for_http_health(&health_client, management_url, "management"),
            wait_for_http_health(&health_client, inference_url, "inference"),
            wait_for_http_health(&health_client, compat_openai_url, "compat-openai"),
        )
    };

    timeout(BALANCER_READY_TIMEOUT, probe)
        .await
        .map_err(|_| {
            anyhow!("balancer services did not become reachable within {BALANCER_READY_TIMEOUT:?}")
        })??;

    Ok(())
}

async fn wait_for_http_health(
    health_client: &reqwest::Client,
    service_url: &Url,
    service_name: &'static str,
) -> Result<()> {
    let health_url = service_url
        .join("/health")
        .map_err(|error| anyhow!("failed to build /health url for {service_name}: {error}"))?;

    loop {
        match health_client.get(health_url.clone()).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => tokio::time::sleep(HEALTH_PROBE_BACKOFF).await,
        }
    }
}
