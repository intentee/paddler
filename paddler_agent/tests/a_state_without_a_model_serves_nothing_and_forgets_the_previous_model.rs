use std::collections::BTreeMap;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

use crate::arbiter_under_test::ArbiterUnderTest;

#[tokio::test]
async fn a_state_without_a_model_serves_nothing_and_forgets_the_previous_model() {
    let mut arbiter_under_test = ArbiterUnderTest::new(CancellationToken::new());

    arbiter_under_test
        .inference_runtime_context
        .model_metadata_holder
        .set_model_metadata(Arc::new(ModelMetadata {
            metadata: BTreeMap::from([("general.name".to_owned(), "previous".to_owned())]),
        }));
    arbiter_under_test
        .pipeline_arbiter
        .apply(AgentApplicableState::NotConfigured);

    assert_eq!(
        arbiter_under_test.conclude_next_progress().await,
        AgentStateApplicationStatus::Fresh
    );
    assert_eq!(
        arbiter_under_test.conclude_next_progress().await,
        AgentStateApplicationStatus::Applied
    );
    assert_eq!(
        arbiter_under_test
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .runtime,
        AgentRuntimeStatus::Idle
    );
    assert_eq!(
        arbiter_under_test
            .inference_runtime_context
            .model_metadata_holder
            .get_model_metadata(),
        None
    );

    arbiter_under_test.pipeline_arbiter.shut_down().await;
}
