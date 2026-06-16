use crate::agent_spawner::AgentSpawner;
use crate::running_balancer::RunningBalancer;

pub struct ProvisionedBackend {
    pub agent_spawner: Box<dyn AgentSpawner>,
    pub running_balancer: RunningBalancer,
}
