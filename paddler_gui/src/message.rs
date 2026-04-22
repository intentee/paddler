use crate::agent_running_handler;
use crate::home_handler;
use crate::join_cluster_config_handler;
use crate::running_cluster_handler;
use crate::start_cluster_config_handler;

#[derive(Debug, Clone)]
pub enum Message {
    Home(home_handler::Message),
    StartClusterConfig(start_cluster_config_handler::Message),
    JoinClusterConfig(join_cluster_config_handler::Message),
    RunningCluster(running_cluster_handler::Message),
    AgentRunning(agent_running_handler::Message),
    ClusterStarted,
    ClusterStopped,
    ClusterFailed(String),
    AgentStopped,
    AgentFailed(String),
    Quit,
    TabPressed { shift: bool },
}
