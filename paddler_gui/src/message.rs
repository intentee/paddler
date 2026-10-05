use crate::agent_running_handler;
use crate::home_handler;
use crate::join_balancer_form_handler;
use crate::running_balancer_handler;
use crate::start_balancer_form_handler;

#[derive(Debug, Clone)]
pub enum Message {
    Home(home_handler::Message),
    StartBalancerForm(start_balancer_form_handler::Message),
    JoinBalancerForm(join_balancer_form_handler::Message),
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
