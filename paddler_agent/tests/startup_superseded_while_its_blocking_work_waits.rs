use std::sync::Arc;

use futures_util::FutureExt as _;
use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;

use crate::arbiter_under_test::ArbiterUnderTest;

pub struct StartupSupersededWhileItsBlockingWorkWaits {
    pub superseded_state: AgentApplicableState,
}

impl StartupSupersededWhileItsBlockingWorkWaits {
    pub fn run(self) -> Arc<SlotAggregatedStatus> {
        single_blocking_thread_runtime()
            .expect("the test runtime must build")
            .block_on(self.run_on_one_blocking_thread())
    }

    async fn run_on_one_blocking_thread(self) -> Arc<SlotAggregatedStatus> {
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
            .await
            .expect("the blocking thread occupant must be released");
        arbiter_under_test.conclude_next_progress().await;
        arbiter_under_test.conclude_next_progress().await;
        arbiter_under_test.conclude_next_progress().await;

        let slot_aggregated_status = arbiter_under_test
            .inference_runtime_context
            .slot_aggregated_status
            .clone();

        arbiter_under_test.pipeline_arbiter.shut_down().await;

        slot_aggregated_status
    }
}
