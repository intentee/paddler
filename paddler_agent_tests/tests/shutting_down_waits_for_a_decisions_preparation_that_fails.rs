use futures_util::FutureExt as _;
use log::Level;
use tokio::join;
use tokio_util::sync::CancellationToken;

use paddler_agent_tests::arbiter_under_test::ArbiterUnderTest;
use paddler_agent_tests::slots_insufficient_for_decisions::slots_insufficient_for_decisions;
use paddler_agent_tests::state_enabling_decisions::state_enabling_decisions;
use paddler_blocking_thread_fixture::occupied_blocking_thread::OccupiedBlockingThread;
use paddler_blocking_thread_fixture::single_blocking_thread_runtime::single_blocking_thread_runtime;
use paddler_capturing_logger::captured_record::CapturedRecord;
use paddler_capturing_logger::capturing_logger::CapturingLogger;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;

#[test]
fn shutting_down_waits_for_a_decisions_preparation_that_fails() {
    CapturingLogger::install();

    single_blocking_thread_runtime()
        .expect("the test runtime must build")
        .block_on(async {
            let occupied_blocking_thread = OccupiedBlockingThread::occupy();
            let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

            arbiter_under_test
                .pipeline_arbiter
                .apply(state_enabling_decisions());

            assert!(
                arbiter_under_test
                    .pipeline_arbiter
                    .next_finished_startup()
                    .now_or_never()
                    .is_none()
            );

            let ArbiterUnderTest {
                inference_runtime_context,
                pipeline_arbiter,
                ..
            } = arbiter_under_test;

            join!(pipeline_arbiter.shut_down(), async {
                occupied_blocking_thread
                    .release()
                    .await
                    .expect("the blocking thread occupant must be released");
            });

            assert!(
                inference_runtime_context.slot_aggregated_status.has_issue(
                    &slots_insufficient_for_decisions(
                        inference_runtime_context
                            .slot_aggregated_status
                            .desired_slots_total
                    )
                )
            );
            assert!(
                CapturingLogger::records_for("paddler_agent::serving_startup").contains(&CapturedRecord {
                    level: Level::Error,
                    message: format!(
                        "Failed to prepare the serving that the shutdown stopped: the decision pipeline failed: decisions need {} slots, the agent runs {}",
                        DECISION_SLOTS_MINIMUM.get(),
                        inference_runtime_context
                            .slot_aggregated_status
                            .desired_slots_total
                    ),
                    target: "paddler_agent::serving_startup".to_owned(),
                })
            );
        });
}
