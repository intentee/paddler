use std::sync::Arc;

use paddler_balancer_runner::balancer_runner_error::BalancerRunnerError;
use paddler_service_thread::service_thread_error::ServiceThreadError;

pub enum RunnerFailure {
    Agent(Arc<ServiceThreadError>),
    Balancer(Arc<BalancerRunnerError>),
}

impl RunnerFailure {
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Agent(service_thread_error) => service_thread_error.to_string(),
            Self::Balancer(balancer_runner_error) => balancer_runner_error.to_string(),
        }
    }
}
