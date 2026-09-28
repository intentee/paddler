use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use tokio::sync::watch;

use crate::balancer_bootstrap_config::BalancerBootstrapConfig;
use crate::balancer_runner_params::BalancerRunnerParams;
use crate::balancer_service_bundle::BalancerServiceBundle;
use crate::bootstrap_error::BootstrapError;
use crate::run_service_manager::run_service_manager;
use crate::service_thread::ServiceThread;

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
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            cancellation_token,
            shutdown_options,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        }: BalancerRunnerParams,
    ) -> Result<Self, BootstrapError> {
        let bundle = BalancerServiceBundle::new(BalancerBootstrapConfig {
            buffered_request_timeout,
            inference_service_configuration,
            management_service_configuration,
            max_buffered_requests,
            openai_service_configuration,
            state_database_type,
            statsd_prefix,
            statsd_service_configuration,
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration,
        })
        .await?;

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

    pub fn wait_for_completion(&mut self) -> impl Future<Output = Result<()>> + Send + 'static {
        self.thread.wait_for_completion()
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
