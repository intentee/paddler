use std::collections::BTreeMap;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::run_scheduler_unless_startup_abandoned::run_scheduler_unless_startup_abandoned;
use paddler_agent_runtime::scheduler_ready::SchedulerReady;
use paddler_agent_runtime::start_scheduler_thread::start_scheduler_thread;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::number_request_preparer::NumberRequestPreparer;
use crate::reports_ready_model_metadata::ReportsReadyModelMetadata as _;

#[tokio::test]
async fn a_scheduler_that_signals_readiness_starts() {
    let NumberRequestPreparer {
        request_preparer, ..
    } = NumberRequestPreparer::new();
    let signalled_model_metadata = ModelMetadata {
        metadata: BTreeMap::from([("general.name".to_owned(), "signalled".to_owned())]),
    };
    let handed_over_model_metadata = signalled_model_metadata.clone();

    let spawn_outcome = start_scheduler_thread::<_, String, _>(
        &CancellationToken::new(),
        move |_agent_shutdown, scheduler_startup_tx| {
            run_scheduler_unless_startup_abandoned(
                scheduler_startup_tx,
                Ok(SchedulerReady {
                    model_metadata: handed_over_model_metadata,
                    request_preparer,
                }),
                || {},
            );

            Ok(())
        },
    )
    .await
    .expect("a scheduler that signalled readiness must start");

    assert_eq!(
        spawn_outcome.ready_model_metadata(),
        Some(&signalled_model_metadata)
    );
}
