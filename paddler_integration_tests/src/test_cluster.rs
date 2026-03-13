use anyhow::Result;
use tempfile::NamedTempFile;

use crate::BALANCER_INFERENCE_ADDR;
use crate::BALANCER_MANAGEMENT_ADDR;
use crate::BALANCER_OPENAI_ADDR;
use crate::balancer_params;
use crate::balancer_params_with_openai;
use crate::managed_agent::ManagedAgent;
use crate::managed_agent::ManagedAgentParams;
use crate::managed_balancer::ManagedBalancer;
use crate::test_cluster_params::TestClusterParams;

pub struct TestCluster {
    pub balancer: ManagedBalancer,
    pub agent: ManagedAgent,
    pub openai_base_url: Option<String>,
    _state_db: NamedTempFile,
}

impl TestCluster {
    pub async fn spawn(params: TestClusterParams) -> Result<Self> {
        let state_db = NamedTempFile::new()?;
        let state_db_url = format!(
            "file://{}",
            state_db
                .path()
                .to_str()
                .expect("temp file path must be valid UTF-8")
        );

        let mut balancer_params = if params.with_openai {
            balancer_params_with_openai(
                BALANCER_MANAGEMENT_ADDR,
                BALANCER_INFERENCE_ADDR,
                BALANCER_OPENAI_ADDR,
                &state_db_url,
                params.max_buffered_requests,
                params.buffered_request_timeout,
            )
        } else {
            balancer_params(
                BALANCER_MANAGEMENT_ADDR,
                BALANCER_INFERENCE_ADDR,
                &state_db_url,
                params.max_buffered_requests,
                params.buffered_request_timeout,
            )
        };

        balancer_params.inference_item_timeout = params.inference_item_timeout;

        let balancer = ManagedBalancer::spawn(balancer_params).await?;

        balancer
            .client()
            .management()
            .put_balancer_desired_state(&params.desired_state)
            .await?;

        balancer.wait_for_desired_state(&params.desired_state).await;

        let agent = ManagedAgent::spawn(ManagedAgentParams {
            management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
            name: Some(params.agent_name),
            slots: params.agent_slots,
        })
        .await?;

        balancer.wait_for_agent_count(1).await;

        if params.wait_for_slots {
            balancer.wait_for_total_slots(params.agent_slots).await;
        }

        let openai_base_url = if params.with_openai {
            Some(format!("http://{BALANCER_OPENAI_ADDR}"))
        } else {
            None
        };

        Ok(Self {
            balancer,
            agent,
            openai_base_url,
            _state_db: state_db,
        })
    }
}
