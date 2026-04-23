use crate::agent_running_handler;
use crate::home_handler;
use crate::join_balancer_config_handler;
use crate::running_balancer_handler;
use crate::start_balancer_config_handler;

#[derive(Debug, Clone)]
pub enum Message {
    Home(home_handler::Message),
    StartBalancerConfig(start_balancer_config_handler::Message),
    JoinBalancerConfig(join_balancer_config_handler::Message),
    RunningBalancer(running_balancer_handler::Message),
    AgentRunning(agent_running_handler::Message),
    BalancerStarted,
    BalancerStopped,
    BalancerFailed(String),
    AgentStopped,
    AgentFailed(String),
    IcedEventLoopReady,
    Quit,
    TabPressed { shift: bool },
}
