use std::sync::Arc;

use anyhow::Result;
use log::debug;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::bootstrap_balancer_params::BootstrapBalancerParams;
use crate::bootstrapped_balancer_handle::bootstrap_balancer;
use crate::service_thread::ServiceThread;

pub struct BootstrappedBalancerBundle {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub initial_desired_state: BalancerDesiredState,
}

pub struct BalancerRunnerParams {
    pub bootstrap_params: BootstrapBalancerParams,
    pub initial_desired_state: Option<BalancerDesiredState>,
    pub parent_shutdown: Option<CancellationToken>,
}

pub struct BalancerRunner {
    initial_bundle_rx: Option<oneshot::Receiver<Arc<BootstrappedBalancerBundle>>>,
    thread: ServiceThread,
}

impl BalancerRunner {
    #[must_use]
    pub fn start(params: BalancerRunnerParams) -> Self {
        let BalancerRunnerParams {
            bootstrap_params,
            initial_desired_state,
            parent_shutdown,
        } = params;

        let (bundle_tx, bundle_rx) = oneshot::channel::<Arc<BootstrappedBalancerBundle>>();

        let thread = ServiceThread::spawn(parent_shutdown, move |task_shutdown| async move {
            let bootstrapped = bootstrap_balancer(bootstrap_params).await?;

            let effective_initial_desired_state = match &initial_desired_state {
                Some(state) => {
                    bootstrapped
                        .state_database
                        .store_balancer_desired_state(state)
                        .await?;

                    state.clone()
                }
                None => {
                    bootstrapped
                        .state_database
                        .read_balancer_desired_state()
                        .await?
                }
            };

            let bundle = Arc::new(BootstrappedBalancerBundle {
                agent_controller_pool: bootstrapped.agent_controller_pool.clone(),
                balancer_applicable_state_holder: bootstrapped
                    .balancer_applicable_state_holder
                    .clone(),
                balancer_desired_state_rx: bootstrapped.balancer_desired_state_tx.subscribe(),
                initial_desired_state: effective_initial_desired_state,
            });

            if bundle_tx.send(bundle).is_err() {
                debug!("balancer runner bundle receiver dropped; continuing without publishing");
            }

            bootstrapped
                .service_manager
                .run_forever(task_shutdown)
                .await
        });

        Self {
            initial_bundle_rx: Some(bundle_rx),
            thread,
        }
    }

    pub const fn take_initial_bundle_rx(
        &mut self,
    ) -> Option<oneshot::Receiver<Arc<BootstrappedBalancerBundle>>> {
        self.initial_bundle_rx.take()
    }

    pub const fn take_completion_rx(&mut self) -> Option<oneshot::Receiver<Result<()>>> {
        self.thread.take_completion_rx()
    }

    pub fn cancel(&self) {
        self.thread.cancel();
    }
}
