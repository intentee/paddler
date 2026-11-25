#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template_issue::UncompilableChatTemplateIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[tokio::test(flavor = "multi_thread")]
async fn a_retried_startup_reuses_the_model_its_failed_attempt_loaded() {
    let model_path = ModelPath::from(
        qwen3_0_6b()
            .cached_path()
            .expect("the Qwen3 model must be in the Hugging Face cache")
            .as_path(),
    );
    let uncompilable_chat_template_issue = UncompilableChatTemplateIssue {
        model_path: model_path.clone(),
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(desired_state_with_chat_template_override(
            qwen3_desired_state(),
            uncompilable_chat_template(),
        ))
        .await
        .expect("the desired state with an uncompilable chat template must be reconciled");
    serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .register_fix(&AgentIssueFix::ChatTemplateIsCompiled(model_path));
    serving_pipeline_arbiter.conclude_next_startup().await;

    assert!(
        serving_pipeline_arbiter
            .inference_runtime_context
            .slot_aggregated_status
            .has_issue_like(|issue| uncompilable_chat_template_issue.is_reported_by(issue))
    );

    serving_pipeline_arbiter.shut_down().await;
}
