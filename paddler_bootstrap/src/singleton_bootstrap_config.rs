use crate::agent_runtime_config::AgentRuntimeConfig;
use crate::balancer_bootstrap_config::BalancerBootstrapConfig;

pub struct SingletonBootstrapConfig {
    pub agent_runtime: AgentRuntimeConfig,
    pub balancer: BalancerBootstrapConfig,
}
