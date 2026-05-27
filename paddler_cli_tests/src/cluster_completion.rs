use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use tokio::process::Child;

pub enum ClusterCompletion {
    InProcess {
        agents: Vec<AgentRunner>,
        balancer: BalancerRunner,
    },
    Subprocess {
        agents: Vec<Child>,
        balancer: Child,
    },
}
