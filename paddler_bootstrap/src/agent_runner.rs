use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use paddler_agent::slot_aggregated_status::SlotAggregatedStatus;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use crate::agent_service_bundle::AgentServiceBundle;
use crate::run_service_manager::run_service_manager;
use crate::service_thread::ServiceThread;

pub struct AgentRunnerParams {
    pub agent_name: Option<String>,
    pub cancellation_token: CancellationToken,
    pub management_address: String,
    pub slots: i32,
}

pub struct AgentRunner {
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    thread: ServiceThread,
}

impl AgentRunner {
    #[must_use]
    pub fn start(
        AgentRunnerParams {
            agent_name,
            cancellation_token,
            management_address,
            slots,
        }: AgentRunnerParams,
    ) -> Self {
        let bundle = AgentServiceBundle::new(agent_name, &management_address, slots);
        let slot_aggregated_status = bundle.slot_aggregated_status.clone();

        let thread = ServiceThread::spawn(cancellation_token, move |task_shutdown| {
            run_service_manager(bundle, task_shutdown, ServiceShutdownOptions::default())
        });

        Self {
            slot_aggregated_status,
            thread,
        }
    }

    pub fn wait_for_completion(&mut self) -> impl Future<Output = Result<()>> + Send + 'static {
        self.thread.wait_for_completion()
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
