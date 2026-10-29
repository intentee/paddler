use std::path::Path;

use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_messaging::agent_desired_state::AgentDesiredState;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_source::agent_local_model_path::AgentLocalModelPath;
use paddler_model_source::model_source::ModelSource;

use crate::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn a_desired_state_with_a_missing_model_is_left_pending_without_starting_a_pipeline() {
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(1);
    let desired_state = BalancerDesiredState {
        model: ModelSource::LocalToAgent(
            AgentLocalModelPath::from_absolute_path(Path::new(
                "/paddler-nonexistent-model-for-the-serving-pipeline.gguf",
            ))
            .expect("the model path must be absolute"),
        )
        .into_agent_desired_model(),
        ..BalancerDesiredState::default()
    };

    assert_eq!(
        serving_pipeline_arbiter.apply(desired_state.clone()).await,
        DesiredStateReconciliation::Pending(Box::new(AgentDesiredState::from(&desired_state)))
    );
    assert_eq!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .make_snapshot()
            .status
            .runtime,
        AgentRuntimeStatus::Idle
    );

    serving_pipeline_arbiter.shut_down().await;
}
