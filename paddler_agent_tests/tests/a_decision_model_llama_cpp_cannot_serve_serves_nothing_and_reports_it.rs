#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::model_with_patched_metadata::ModelWithPatchedMetadata;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_capability_failure_params::ModelCapabilityFailureParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_llama_cpp_cannot_serve_serves_nothing_and_reports_it() {
    let decision_model_without_its_decision_metadata =
        ModelWithPatchedMetadata::write(smolvlm2_256m(), |gguf_context| {
            gguf_context.set_val_str(c"llama.decision.type", c"lev");

            Ok(())
        })
        .expect("the model declaring a lev decision type must be written");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(2).expect("the model downloader must start");

    serving_pipeline_arbiter
        .apply(
            decision_model_without_its_decision_metadata
                .desired_state()
                .expect("the decision model must lie at an absolute path"),
        )
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;
    let model_path = ModelPath::from(
        decision_model_without_its_decision_metadata
            .model_path()
            .as_path(),
    );

    assert!(status.issues.iter().any(|issue| matches!(
        issue,
        AgentIssue::DecisionModelCannotBeServed(ModelCapabilityFailureParams {
            model_path: issue_model_path,
            ..
        }) if *issue_model_path == model_path
    )));
    assert_eq!(status.runtime, AgentRuntimeStatus::Idle);

    serving_pipeline_arbiter.shut_down().await;
}
