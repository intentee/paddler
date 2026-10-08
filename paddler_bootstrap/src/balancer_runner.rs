use tokio::sync::watch;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::balancer_runner_params::BalancerRunnerParams;
use crate::balancer_service_bundle::BalancerServiceBundle;
use crate::bootstrap_error::BootstrapError;
use crate::run_service_manager::run_service_manager;
use crate::service_thread::ServiceThread;

pub struct BalancerRunner {
    pub addresses: BalancerAddresses,
    pub balancer_desired_state_tx: watch::Sender<BalancerDesiredState>,
    thread: ServiceThread,
}

impl BalancerRunner {
    pub async fn start(
        BalancerRunnerParams {
            bootstrap_config,
            cancellation_token,
            shutdown_options,
        }: BalancerRunnerParams,
    ) -> Result<Self, BootstrapError> {
        let bundle = BalancerServiceBundle::new(bootstrap_config).await?;

        let addresses = bundle.addresses;
        let balancer_desired_state_tx = bundle.balancer_desired_state_tx.clone();

        let thread = ServiceThread::spawn(cancellation_token, move |task_shutdown| {
            run_service_manager(bundle, task_shutdown, shutdown_options)
        });

        Ok(Self {
            addresses,
            balancer_desired_state_tx,
            thread,
        })
    }

    pub async fn wait_for_completion(self) -> Result<(), BootstrapError> {
        self.thread.wait_for_completion().await
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
