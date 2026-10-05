use std::sync::Arc;

use anyhow::Result;
use tokio::sync::watch;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::balancer_connection::BalancerConnection;

use crate::agent_runner_params::AgentRunnerParams;
use crate::agent_service_bundle::AgentServiceBundle;
use crate::bootstrap_error::BootstrapError;
use crate::run_service_manager::run_service_manager;
use crate::service_thread::ServiceThread;

pub struct AgentRunner {
    pub balancer_connection_rx: watch::Receiver<BalancerConnection>,
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    thread: ServiceThread,
}

impl AgentRunner {
    #[must_use]
    pub fn start(
        AgentRunnerParams {
            bootstrap_config,
            cancellation_token,
            shutdown_options,
        }: AgentRunnerParams,
    ) -> Self {
        let bundle = AgentServiceBundle::new(bootstrap_config);
        let balancer_connection_rx = bundle.balancer_connection_rx.clone();
        let slot_aggregated_status = bundle.slot_aggregated_status.clone();

        let thread = ServiceThread::spawn(cancellation_token, move |task_shutdown| {
            run_service_manager(bundle, task_shutdown, shutdown_options)
        });

        Self {
            balancer_connection_rx,
            slot_aggregated_status,
            thread,
        }
    }

    pub async fn wait_for_completion(self) -> Result<(), BootstrapError> {
        self.thread.wait_for_completion().await
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
