use std::future::Future;
use std::future::pending;
use std::pin::Pin;

use log::error;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;
use paddler_messaging::agent_desired_state::AgentDesiredState;

use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;
use crate::retry_delay::RetryDelay;

pub enum DesiredStateReconciliationAttempt {
    Idle,
    Reconciling {
        cancellation_token: CancellationToken,
        reconciliation: Pin<Box<dyn Future<Output = DesiredStateReconciliation> + Send>>,
    },
}

impl DesiredStateReconciliationAttempt {
    #[must_use]
    pub fn start(
        desired_state_reconciler: DesiredStateReconciler,
        agent_desired_state: AgentDesiredState,
        agent_shutdown: &CancellationToken,
        retry_delay: RetryDelay,
    ) -> Self {
        let cancellation_token = agent_shutdown.child_token();
        let reconciliation_cancellation_token = cancellation_token.clone();

        Self::Reconciling {
            cancellation_token,
            reconciliation: Box::pin(async move {
                retry_delay.elapse(&reconciliation_cancellation_token).await;

                match Builder::new_current_thread().enable_all().build() {
                    Ok(reconciliation_runtime) => {
                        spawn_blocking_resuming_panic(move || {
                            reconciliation_runtime.block_on(
                                desired_state_reconciler.reconcile(
                                    &reconciliation_cancellation_token,
                                    agent_desired_state,
                                ),
                            )
                        })
                        .await
                    }
                    Err(runtime_creation_error) => {
                        error!(
                            "Unable to create the runtime that reconciles the desired state: {runtime_creation_error}"
                        );

                        DesiredStateReconciliation::Pending(Box::new(agent_desired_state))
                    }
                }
            }),
        }
    }

    pub fn abandon(&self) {
        if let Self::Reconciling {
            cancellation_token, ..
        } = self
        {
            cancellation_token.cancel();
        }
    }

    pub async fn finish(&mut self) -> DesiredStateReconciliation {
        match self {
            Self::Idle => pending().await,
            Self::Reconciling { reconciliation, .. } => {
                let desired_state_reconciliation = reconciliation.as_mut().await;

                *self = Self::Idle;

                desired_state_reconciliation
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::os::fd::AsRawFd as _;
    use std::sync::Arc;

    use nix::sys::resource::Resource;
    use nix::sys::resource::getrlimit;
    use nix::sys::resource::setrlimit;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_messaging::agent_desired_state::AgentDesiredState;

    use super::DesiredStateReconciliationAttempt;
    use crate::agent_applicable_state_holder::AgentApplicableStateHolder;
    use crate::desired_state_reconciler::DesiredStateReconciler;
    use crate::desired_state_reconciliation::DesiredStateReconciliation;
    use crate::retry_delay::RetryDelay;

    #[tokio::test]
    async fn a_reconciliation_that_cannot_create_its_runtime_stays_pending() {
        let (soft_descriptor_limit, hard_descriptor_limit) =
            getrlimit(Resource::RLIMIT_NOFILE).unwrap();
        let lowest_free_descriptor = File::open("/dev/null").unwrap().as_raw_fd();
        let mut reconciliation_attempt = DesiredStateReconciliationAttempt::start(
            DesiredStateReconciler {
                agent_applicable_state_holder: Arc::new(AgentApplicableStateHolder::default()),
                cache_dir: CacheDir::from_process_env(),
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
            },
            AgentDesiredState::default(),
            &CancellationToken::new(),
            RetryDelay::Immediately,
        );

        setrlimit(
            Resource::RLIMIT_NOFILE,
            u64::try_from(lowest_free_descriptor).unwrap(),
            hard_descriptor_limit,
        )
        .unwrap();

        let desired_state_reconciliation = reconciliation_attempt.finish().await;

        setrlimit(
            Resource::RLIMIT_NOFILE,
            soft_descriptor_limit,
            hard_descriptor_limit,
        )
        .unwrap();

        assert_eq!(
            desired_state_reconciliation,
            DesiredStateReconciliation::Pending(Box::default())
        );
    }
}
