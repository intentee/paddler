use anyhow::Result;
use anyhow::anyhow;
use tempfile::NamedTempFile;

use crate::BALANCER_INFERENCE_ADDR;
use crate::BALANCER_MANAGEMENT_ADDR;
use crate::BALANCER_OPENAI_ADDR;
use crate::managed_agent::ManagedAgent;
use crate::managed_agent::ManagedAgentParams;
use crate::managed_balancer::ManagedBalancer;
use crate::managed_balancer::ManagedBalancerParams;
use crate::managed_cluster_params::ManagedClusterParams;

pub struct ManagedCluster {
    pub balancer: ManagedBalancer,
    pub agent: ManagedAgent,
    pub openai_base_url: String,
    _state_db: NamedTempFile,
}

impl ManagedCluster {
    pub async fn spawn(params: ManagedClusterParams) -> Result<Self> {
        let state_db = NamedTempFile::new()?;
        let state_db_path = state_db
            .path()
            .to_str()
            .ok_or_else(|| anyhow!("temp file path is not valid UTF-8"))?;
        let state_db_url = format!("file://{state_db_path}");

        let balancer_params = ManagedBalancerParams {
            buffered_request_timeout: params.buffered_request_timeout,
            compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
            inference_addr: BALANCER_INFERENCE_ADDR.to_owned(),
            inference_cors_allowed_hosts: vec![],
            inference_item_timeout: params.inference_item_timeout,
            management_addr: BALANCER_MANAGEMENT_ADDR.to_owned(),
            management_cors_allowed_hosts: vec![],
            max_buffered_requests: params.max_buffered_requests,
            state_database_url: state_db_url,
        };

        let balancer = ManagedBalancer::spawn(balancer_params).await?;

        balancer
            .client()
            .management()
            .put_balancer_desired_state(&params.desired_state)
            .await?;

        balancer.wait_for_desired_state(&params.desired_state).await;

        let agent = ManagedAgent::spawn(&ManagedAgentParams {
            management_addr: BALANCER_MANAGEMENT_ADDR.to_owned(),
            name: Some(params.agent_name),
            slots: params.agent_slots,
        })?;

        balancer.wait_for_agent_count(1).await;

        if params.wait_for_slots {
            balancer.wait_for_total_slots(params.agent_slots).await;
        }

        let openai_base_url = format!("http://{BALANCER_OPENAI_ADDR}");

        Ok(Self {
            balancer,
            agent,
            openai_base_url,
            _state_db: state_db,
        })
    }
}
