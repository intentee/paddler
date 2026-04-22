use std::sync::Arc;

use anyhow::Result;
use log::debug;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::bootstrap_agent_params::BootstrapAgentParams;
use crate::bootstrapped_agent_handle::bootstrap_agent;
use crate::service_thread::ServiceThread;

pub struct AgentRunnerParams {
    pub bootstrap_params: BootstrapAgentParams,
    pub parent_shutdown: Option<CancellationToken>,
}

pub struct AgentRunner {
    initial_status_rx: Option<oneshot::Receiver<Arc<SlotAggregatedStatus>>>,
    thread: ServiceThread,
}

impl AgentRunner {
    #[must_use]
    pub fn start(params: AgentRunnerParams) -> Self {
        let AgentRunnerParams {
            bootstrap_params,
            parent_shutdown,
        } = params;

        let (status_tx, status_rx) = oneshot::channel::<Arc<SlotAggregatedStatus>>();

        let thread = ServiceThread::spawn(parent_shutdown, move |task_shutdown| async move {
            let bootstrapped = bootstrap_agent(bootstrap_params);

            if status_tx
                .send(bootstrapped.slot_aggregated_status.clone())
                .is_err()
            {
                debug!("agent runner status receiver dropped; continuing without publishing");
            }

            bootstrapped
                .service_manager
                .run_forever(task_shutdown)
                .await
        });

        Self {
            initial_status_rx: Some(status_rx),
            thread,
        }
    }

    pub const fn take_initial_status_rx(
        &mut self,
    ) -> Option<oneshot::Receiver<Arc<SlotAggregatedStatus>>> {
        self.initial_status_rx.take()
    }

    pub const fn take_completion_rx(&mut self) -> Option<oneshot::Receiver<Result<()>>> {
        self.thread.take_completion_rx()
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
