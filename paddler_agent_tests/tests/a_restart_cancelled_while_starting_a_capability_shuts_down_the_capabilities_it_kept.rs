#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent::finished_serving_startup::FinishedServingStartup;
use paddler_agent::inference_capabilities::InferenceCapabilities;
use paddler_agent::model_serving::ModelServing;
use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent::replacement_outcome::ReplacementOutcome;
use paddler_agent::retry_delay::RetryDelay;
use paddler_agent::serving_replacement::ServingReplacement;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::model_serving_text_generation_and_embeddings::ModelServingTextGenerationAndEmbeddings;
use paddler_agent_pipeline_harness::raw_prompt_generation::RawPromptGeneration;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_cancelled_while_starting_a_capability_shuts_down_the_capabilities_it_kept() {
    let ModelServingTextGenerationAndEmbeddings {
        llama_runtime:
            LlamaRuntime {
                inference_runtime_context,
                loaded_llama_model: model_loaded_for_serving,
                ..
            },
        model_serving_settings,
        served_model,
    } = ModelServingTextGenerationAndEmbeddings::start(&CancellationToken::new())
        .await
        .expect("the model must load")
        .expect("text generation and embeddings must start");

    drop(model_loaded_for_serving);

    let kept_text_generation_request_preparer = served_model
        .capabilities
        .text_generation
        .request_preparer
        .clone();
    let cancelled_restart = CancellationToken::new();

    cancelled_restart.cancel();

    let FinishedServingStartup {
        outcome: restart_outcome,
        ..
    } = ServingReplacement {
        agent_applicable_state: AgentApplicableState::WithModel(ModelServingSettings {
            capabilities: InferenceCapabilities {
                embeddings: PoolingType::Mean,
                ..model_serving_settings.capabilities.clone()
            },
            ..model_serving_settings
        }),
        held: ModelServing::Serving(Box::new(served_model)),
        retry_delay: RetryDelay::Immediately,
    }
    .replace(cancelled_restart, inference_runtime_context.clone())
    .await;

    assert_eq!(
        discriminant(&restart_outcome),
        discriminant(&ReplacementOutcome::Cancelled {
            held: ModelServing::Idle
        })
    );
    assert!(
        inference_runtime_context
            .vram_residencies
            .text_generation_context
            .try_occupy()
            .is_ok()
    );

    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = RawPromptGeneration::new(
        "The capital of France is",
        inference_runtime_context.slot_aggregated_status.clone(),
    );

    kept_text_generation_request_preparer
        .prepare(TextGenerationRequest::ContinueFromRawPrompt(agent_request));

    assert!(matches!(
        generated_tokens_rx.recv().await,
        Some(GeneratedTokenResult::SchedulerUnavailable(_))
    ));

    kept_text_generation_request_preparer
        .preparation_tasks
        .wait()
        .await;
    drop(kept_text_generation_request_preparer);

    let model_weights = &inference_runtime_context.vram_residencies.model_weights;

    assert!(model_weights.try_occupy().is_err());

    drop(restart_outcome);

    assert!(model_weights.try_occupy().is_ok());
}
