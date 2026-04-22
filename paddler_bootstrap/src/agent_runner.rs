use std::sync::Arc;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;
use log::error;
use paddler::slot_aggregated_status::SlotAggregatedStatus;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::bootstrap_agent_params::BootstrapAgentParams;
use crate::bootstrapped_agent_handle::bootstrap_agent;

pub struct AgentRunnerParams {
    pub bootstrap_params: BootstrapAgentParams,
    pub parent_shutdown: Option<CancellationToken>,
}

pub struct AgentRunner {
    initial_status_rx: Option<oneshot::Receiver<Arc<SlotAggregatedStatus>>>,
    shutdown: CancellationToken,
    thread: Option<thread::JoinHandle<Result<()>>>,
}

impl AgentRunner {
    pub fn start(params: AgentRunnerParams) -> Self {
        let AgentRunnerParams {
            bootstrap_params,
            parent_shutdown,
        } = params;

        let shutdown = parent_shutdown
            .as_ref()
            .map_or_else(CancellationToken::new, CancellationToken::child_token);
        let task_shutdown = shutdown.clone();
        let (status_tx, status_rx) = oneshot::channel::<Arc<SlotAggregatedStatus>>();

        let thread = thread::spawn(move || -> Result<()> {
            actix_web::rt::System::new().block_on(async move {
                let bootstrapped = bootstrap_agent(bootstrap_params);

                if status_tx
                    .send(bootstrapped.slot_aggregated_status.clone())
                    .is_err()
                {
                    return Err(anyhow!(
                        "agent runner status receiver dropped before bootstrap completed"
                    ));
                }

                bootstrapped
                    .service_manager
                    .run_forever(task_shutdown)
                    .await
            })
        });

        Self {
            initial_status_rx: Some(status_rx),
            shutdown,
            thread: Some(thread),
        }
    }

    pub const fn take_initial_status_rx(
        &mut self,
    ) -> Option<oneshot::Receiver<Arc<SlotAggregatedStatus>>> {
        self.initial_status_rx.take()
    }

    pub fn cancel(&self) {
        self.shutdown.cancel();
    }
}

impl Drop for AgentRunner {
    fn drop(&mut self) {
        self.shutdown.cancel();

        if let Some(thread) = self.thread.take() {
            match thread.join() {
                Ok(Ok(())) => {}
                Ok(Err(service_error)) => {
                    error!("agent runner exited with error: {service_error}");
                }
                Err(panic_payload) => {
                    error!("agent runner thread panicked: {panic_payload:?}");
                }
            }
        }
    }
}
