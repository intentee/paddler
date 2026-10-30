use std::future::Future;
use std::future::pending;
use std::pin::Pin;

use log::error;
use tokio::runtime::Builder;
use tokio_util::sync::CancellationToken;

use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::desired_state_reconciler::DesiredStateReconciler;
use crate::desired_state_reconciliation::DesiredStateReconciliation;
use crate::retry_delay::RetryDelay;
use crate::retry_delay_outcome::RetryDelayOutcome;

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
                if retry_delay.elapse(&reconciliation_cancellation_token).await
                    == RetryDelayOutcome::Cancelled
                {
                    return DesiredStateReconciliation::Pending(Box::new(agent_desired_state));
                }

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

    pub async fn abandon(&mut self) {
        if let Self::Reconciling {
            cancellation_token,
            reconciliation,
        } = self
        {
            cancellation_token.cancel();
            reconciliation.as_mut().await;
        }

        *self = Self::Idle;
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
    use std::fs::read_dir;
    use std::os::fd::AsRawFd as _;
    use std::sync::Arc;

    use nix::sys::resource::Resource;
    use nix::sys::resource::getrlimit;
    use nix::sys::resource::setrlimit;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_cache_dir::cache_dir::CacheDir;
    use paddler_download_manager::download_manager::DownloadManager;
    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_model_source::desired_model_resolver::DesiredModelResolver;
    use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
    use paddler_model_source::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use paddler_model_source::huggingface_hub::HuggingFaceHub;

    use super::DesiredStateReconciliationAttempt;
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
                desired_model_resolver: DesiredModelResolver::from_process_env(
                    HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                )
                .unwrap(),
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

    #[tokio::test(start_paused = true)]
    async fn a_reconciliation_abandoned_during_its_retry_delay_never_resolves_its_model() {
        let cache_dir = TempDir::new().unwrap();
        let model_server = LocalHttpFixture::start(FixtureResponse::Ok(b"model".to_vec()))
            .await
            .unwrap();
        let mut reconciliation_attempt = DesiredStateReconciliationAttempt::start(
            DesiredStateReconciler {
                desired_model_resolver: DesiredModelResolver {
                    cache_dir: CacheDir {
                        explicit: Some(cache_dir.path().to_path_buf()),
                        home: None,
                        xdg: None,
                    },
                    download_manager: DownloadManager::new().unwrap(),
                    huggingface_client: HuggingFaceHub {
                        cache_location: HuggingFaceCacheLocation::HubDefault,
                        endpoint: HUGGINGFACE_DEFAULT_ENDPOINT.to_owned(),
                    }
                    .client()
                    .unwrap(),
                },
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
            },
            AgentDesiredState {
                model: AgentDesiredModel::Uri(model_server.url("/model.gguf")),
                ..AgentDesiredState::default()
            },
            &CancellationToken::new(),
            RetryDelay::AfterRetryInterval,
        );

        reconciliation_attempt.abandon().await;

        assert_eq!(read_dir(cache_dir.path()).unwrap().count(), 0);
        assert_eq!(model_server.request_count(), 0);

        model_server
            .stop()
            .await
            .expect("the fixture task must not panic")
            .expect("the fixture must serve every request");
    }
}
