#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::model_with_patched_metadata::ModelWithPatchedMetadata;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_capability_failure_params::ModelCapabilityFailureParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

const POOLING_TYPE_LLAMA_CPP_DOES_NOT_DEFINE: u32 = 99;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_declaring_an_unknown_pooling_type_serves_nothing_and_reports_it() {
    let model_declaring_an_unknown_pooling_type =
        ModelWithPatchedMetadata::write(nomic_embed_text_v1_5(), |gguf_context| {
            gguf_context.set_val_u32(
                c"nomic-bert.pooling_type",
                POOLING_TYPE_LLAMA_CPP_DOES_NOT_DEFINE,
            );

            Ok(())
        })
        .expect("the model declaring an unknown pooling type must be written");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .apply(
            model_declaring_an_unknown_pooling_type
                .desired_state()
                .expect("the model must lie at an absolute path"),
        )
        .await;
    serving_pipeline_arbiter.conclude_next_startup().await;

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;
    let model_path = ModelPath::from(
        model_declaring_an_unknown_pooling_type
            .model_path()
            .as_path(),
    );

    assert!(status.issues.iter().any(|issue| matches!(
        issue,
        AgentIssue::ModelDeclaresAnUnknownPoolingType(ModelCapabilityFailureParams {
            model_path: issue_model_path,
            ..
        }) if *issue_model_path == model_path
    )));
    assert_eq!(status.runtime, AgentRuntimeStatus::Idle);

    serving_pipeline_arbiter.shut_down().await;
}
