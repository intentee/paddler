use anyhow::Result;
use anyhow::anyhow;
use tempfile::NamedTempFile;

use crate::managed_agent::ManagedAgent;
use crate::managed_agent::ManagedAgentParams;
use crate::managed_balancer::ManagedBalancer;
use crate::managed_balancer::ManagedBalancerParams;
use crate::managed_cluster_params::ManagedClusterParams;
use crate::pick_free_port::pick_balancer_addresses;

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

        let addresses = pick_balancer_addresses()?;
        let management_addr = addresses.management.clone();
        let compat_openai_addr = addresses.compat_openai.clone();

        let balancer_params = ManagedBalancerParams {
            buffered_request_timeout: params.buffered_request_timeout,
            compat_openai_addr: addresses.compat_openai,
            inference_addr: addresses.inference,
            inference_cors_allowed_hosts: vec![],
            inference_item_timeout: params.inference_item_timeout,
            management_addr: addresses.management,
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

        let expected_applicable_state = params.desired_state.to_agent_desired_state();

        balancer
            .wait_for_applicable_state(&expected_applicable_state)
            .await;

        let agent = ManagedAgent::spawn(&ManagedAgentParams {
            management_addr,
            name: Some(params.agent_name),
            slots: params.agent_slots,
        })?;

        balancer.wait_for_agent_count(1).await;

        if params.wait_for_slots {
            balancer.wait_for_total_slots(params.agent_slots).await;
        }

        let openai_base_url = format!("http://{compat_openai_addr}");

        Ok(Self {
            balancer,
            agent,
            openai_base_url,
            _state_db: state_db,
        })
    }
}
