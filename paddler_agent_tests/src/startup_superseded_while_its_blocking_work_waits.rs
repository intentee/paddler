use std::sync::Arc;

use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;

use crate::agent_tests_error::AgentTestsError;
use crate::arbiter_under_test::ArbiterUnderTest;

async fn conclude_the_superseding_startup(
    mut arbiter_under_test: ArbiterUnderTest,
) -> Result<Arc<SlotAggregatedStatus>, AgentTestsError> {
    arbiter_under_test.conclude_next_progress().await;
    arbiter_under_test.conclude_next_progress().await;
    arbiter_under_test.conclude_next_progress().await;

    let slot_aggregated_status = arbiter_under_test
        .inference_runtime_context
        .slot_aggregated_status
        .clone();

    arbiter_under_test.pipeline_arbiter.shut_down().await;

    Ok(slot_aggregated_status)
}

pub struct StartupSupersededWhileItsBlockingWorkWaits {
    pub superseded_state: AgentApplicableState,
}

impl StartupSupersededWhileItsBlockingWorkWaits {
    /// # Errors
    ///
    /// Returns [`AgentTestsError::TestRuntimeUnbuildable`] when the single blocking thread runtime cannot be built, and [`AgentTestsError::BlockingThreadOccupantUnreleased`] when the blocking thread occupant cannot be released.
    pub fn run(self) -> Result<Arc<SlotAggregatedStatus>, AgentTestsError> {
        single_blocking_thread_runtime()
            .map_err(AgentTestsError::TestRuntimeUnbuildable)
            .and_then(|runtime| runtime.block_on(self.run_on_one_blocking_thread()))
    }

    async fn run_on_one_blocking_thread(
        self,
    ) -> Result<Arc<SlotAggregatedStatus>, AgentTestsError> {
        let occupied_blocking_thread = OccupiedBlockingThread::occupy();
        let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

        arbiter_under_test
            .pipeline_arbiter
            .apply(self.superseded_state);

        while let Some(serving_startup_progress) = arbiter_under_test
            .pipeline_arbiter
            .next_finished_startup()
            .now_or_never()
        {
            arbiter_under_test
                .pipeline_arbiter
                .conclude(serving_startup_progress);
        }

        arbiter_under_test
            .pipeline_arbiter
            .apply(AgentApplicableState::NotConfigured);
        occupied_blocking_thread
            .release()
            .map_err(AgentTestsError::BlockingThreadOccupantUnreleased)
            .and_then(|()| conclude_the_superseding_startup(arbiter_under_test))
            .await
    }
}
