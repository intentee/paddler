use std::sync::Arc;
use std::thread;

use anyhow::Result;
use anyhow::anyhow;
use log::error;
use paddler::balancer::agent_controller_pool::AgentControllerPool;
use paddler::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use tokio::sync::broadcast;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::bootstrap_balancer_params::BootstrapBalancerParams;
use crate::bootstrapped_balancer_handle::bootstrap_balancer;

pub struct BootstrappedBalancerBundle {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub balancer_desired_state_rx: broadcast::Receiver<BalancerDesiredState>,
    pub initial_desired_state: BalancerDesiredState,
}

pub struct ClusterRunnerParams {
    pub bootstrap_params: BootstrapBalancerParams,
    pub initial_desired_state: BalancerDesiredState,
    pub parent_shutdown: Option<CancellationToken>,
}

pub struct ClusterRunner {
    completion_rx: Option<oneshot::Receiver<Result<()>>>,
    initial_bundle_rx: Option<oneshot::Receiver<Arc<BootstrappedBalancerBundle>>>,
    shutdown: CancellationToken,
    thread: Option<thread::JoinHandle<()>>,
}

impl ClusterRunner {
    pub fn start(params: ClusterRunnerParams) -> Self {
        let ClusterRunnerParams {
            bootstrap_params,
            initial_desired_state,
            parent_shutdown,
        } = params;

        let shutdown = parent_shutdown
            .as_ref()
            .map_or_else(CancellationToken::new, CancellationToken::child_token);
        let task_shutdown = shutdown.clone();
        let (bundle_tx, bundle_rx) = oneshot::channel::<Arc<BootstrappedBalancerBundle>>();
        let (completion_tx, completion_rx) = oneshot::channel::<Result<()>>();

        let thread = thread::spawn(move || {
            let result = actix_web::rt::System::new().block_on(async move {
                let bootstrapped = bootstrap_balancer(bootstrap_params).await?;

                let bundle = Arc::new(BootstrappedBalancerBundle {
                    agent_controller_pool: bootstrapped.agent_controller_pool.clone(),
                    balancer_applicable_state_holder: bootstrapped
                        .balancer_applicable_state_holder
                        .clone(),
                    balancer_desired_state_rx: bootstrapped.balancer_desired_state_tx.subscribe(),
                    initial_desired_state: initial_desired_state.clone(),
                });

                if bundle_tx.send(bundle).is_err() {
                    return Err(anyhow!(
                        "cluster runner bundle receiver dropped before bootstrap completed"
                    ));
                }

                bootstrapped
                    .state_database
                    .store_balancer_desired_state(&initial_desired_state)
                    .await?;

                bootstrapped
                    .service_manager
                    .run_forever(task_shutdown)
                    .await
            });

            let _ = completion_tx.send(result);
        });

        Self {
            completion_rx: Some(completion_rx),
            initial_bundle_rx: Some(bundle_rx),
            shutdown,
            thread: Some(thread),
        }
    }

    pub const fn take_initial_bundle_rx(
        &mut self,
    ) -> Option<oneshot::Receiver<Arc<BootstrappedBalancerBundle>>> {
        self.initial_bundle_rx.take()
    }

    pub const fn take_completion_rx(&mut self) -> Option<oneshot::Receiver<Result<()>>> {
        self.completion_rx.take()
    }

    pub fn cancel(&self) {
        self.shutdown.cancel();
    }
}

impl Drop for ClusterRunner {
    fn drop(&mut self) {
        self.shutdown.cancel();

        if let Some(thread) = self.thread.take()
            && let Err(panic_payload) = thread.join()
        {
            error!("cluster runner thread panicked: {panic_payload:?}");
        }
    }
}
