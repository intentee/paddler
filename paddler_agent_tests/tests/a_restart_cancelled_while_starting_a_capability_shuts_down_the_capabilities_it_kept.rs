#![cfg(feature = "tests_that_use_llms")]

use std::mem::discriminant;

use tokio_util::sync::CancellationToken;

use paddler_agent::agent_applicable_state::AgentApplicableState;
use paddler_agent::finished_serving_startup::FinishedServingStartup;
use paddler_agent::model_serving::ModelServing;
use paddler_agent::model_serving_settings::ModelServingSettings;
use paddler_agent::replacement_outcome::ReplacementOutcome;
use paddler_agent::retry_delay::RetryDelay;
use paddler_agent::serving_replacement::ServingReplacement;
use paddler_agent_pipeline_harness::embedding_batch_generation::EmbeddingBatchGeneration;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::model_serving_text_generation_and_embeddings::ModelServingTextGenerationAndEmbeddings;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::desired_sampling_parameters::DesiredSamplingParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_result::EmbeddingResult;

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_cancelled_while_starting_a_capability_shuts_down_the_capabilities_it_kept() {
    let serving = ModelServingTextGenerationAndEmbeddings::start(&CancellationToken::new())
        .await
        .expect("the model must load")
        .expect("text generation and embeddings must start");
    let kept_embeddings_request_preparer = serving
        .embeddings_request_preparer()
        .expect("the served model must serve embeddings");
    let ModelServingTextGenerationAndEmbeddings {
        llama_runtime:
            LlamaRuntime {
                inference_runtime_context,
                loaded_llama_model: model_loaded_for_serving,
                ..
            },
        model_serving_settings,
        served_model,
    } = serving;

    drop(model_loaded_for_serving);
    let cancelled_restart = CancellationToken::new();

    cancelled_restart.cancel();

    let FinishedServingStartup {
        outcome: restart_outcome,
        ..
    } = ServingReplacement {
        agent_applicable_state: AgentApplicableState::WithModel(ModelServingSettings {
            text_generation: TextGenerationSettings {
                sampling_parameters: DesiredSamplingParameters::from(
                    SamplingParameters::deterministic(),
                ),
                ..model_serving_settings.text_generation.clone()
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
            .embeddings_context
            .try_occupy()
            .is_ok()
    );

    let EmbeddingBatchGeneration {
        agent_request,
        generate_embedding_stop_tx: _generate_embedding_stop_tx,
        mut generated_embedding_rx,
    } = EmbeddingBatchGeneration::new(
        vec![EmbeddingInputDocument {
            content: "The capital of France is Paris.".to_owned(),
            id: "capital".to_owned(),
        }],
        inference_runtime_context.slot_aggregated_status.clone(),
    );

    kept_embeddings_request_preparer.prepare(agent_request);

    assert!(matches!(
        generated_embedding_rx.recv().await,
        Some(EmbeddingResult::SchedulerUnavailable(_))
    ));

    kept_embeddings_request_preparer
        .preparation_tasks
        .wait()
        .await;
    drop(kept_embeddings_request_preparer);

    let model_weights = &inference_runtime_context.vram_residencies.model_weights;

    assert!(model_weights.try_occupy().is_err());

    drop(restart_outcome);

    assert!(model_weights.try_occupy().is_ok());
}
