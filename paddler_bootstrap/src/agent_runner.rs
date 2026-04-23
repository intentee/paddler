use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use tokio_util::sync::CancellationToken;

use crate::bootstrapped_agent_handle::BootstrappedAgentHandle;
use crate::bootstrapped_agent_handle::bootstrap_agent;
use crate::service_thread::ServiceThread;

pub struct AgentRunnerParams {
    pub agent_name: Option<String>,
    pub management_address: String,
    pub parent_shutdown: Option<CancellationToken>,
    pub slots: i32,
}

pub struct AgentRunner {
    pub slot_aggregated_status: Arc<SlotAggregatedStatus>,
    thread: ServiceThread,
}

impl AgentRunner {
    #[must_use]
    pub fn start(params: AgentRunnerParams) -> Self {
        let AgentRunnerParams {
            agent_name,
            management_address,
            parent_shutdown,
            slots,
        } = params;

        let BootstrappedAgentHandle {
            service_manager,
            slot_aggregated_status,
        } = bootstrap_agent(agent_name, &management_address, slots);

        let thread = ServiceThread::spawn(parent_shutdown, move |task_shutdown| async move {
            service_manager.run_forever(task_shutdown).await
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
