use std::sync::Arc;

use tokio::sync::watch;

use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_service_thread::run_service_manager::run_service_manager;
use paddler_service_thread::service_thread::ServiceThread;

use crate::balancer_runner_error::BalancerRunnerError;
use crate::balancer_runner_params::BalancerRunnerParams;
use crate::balancer_service_bundle::BalancerServiceBundle;

pub struct BalancerRunner {
    pub addresses: BalancerAddresses,
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_tx: watch::Sender<BalancerDesiredState>,
    thread: ServiceThread,
}

impl BalancerRunner {
    pub async fn start(
        BalancerRunnerParams {
            runner_config,
            cancellation_token,
            shutdown_options,
        }: BalancerRunnerParams,
    ) -> Result<Self, BalancerRunnerError> {
        let bundle = BalancerServiceBundle::new(runner_config).await?;

        let addresses = bundle.addresses;
        let agent_controller_pool = bundle.agent_controller_pool.clone();
        let balancer_applicable_state_holder = bundle.balancer_applicable_state_holder.clone();
        let balancer_desired_state_tx = bundle.balancer_desired_state_tx.clone();

        let thread = ServiceThread::spawn(cancellation_token, move |task_shutdown| {
            run_service_manager(bundle, task_shutdown, shutdown_options)
        });

        Ok(Self {
            addresses,
            agent_controller_pool,
            balancer_applicable_state_holder,
            balancer_desired_state_tx,
            thread,
        })
    }

    pub async fn wait_for_completion(self) -> Result<(), BalancerRunnerError> {
        self.thread
            .wait_for_completion()
            .await
            .map_err(BalancerRunnerError::from)
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
