#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::error::DecodeError;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::loaded_pointer_head::LoadedPointerHead;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_pointer_head::kev_0_8b_pointer_head;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: u16 = 200;
const CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_pipeline_whose_context_cannot_warm_up_every_slot_refuses_to_start() {
    let LlamaRuntime {
        inference_runtime_context,
        loaded_llama_model,
        model_runtime_parameters,
    } = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the kev model must load");

    assert_eq!(
        DecisionPipeline {
            inference_runtime_context,
            loaded_llama_model,
            loaded_pointer_head: LoadedPointerHead {
                pointer_head: kev_0_8b_pointer_head().expect("the kev pointer head must load"),
                pointer_head_path: KevCheckpoint::Kev0_8b.target_path("pointer_head.gguf"),
            },
            model_runtime_parameters: ModelRuntimeParameters {
                context_size: CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT,
                ..model_runtime_parameters
            },
        }
        .spawn(&CancellationToken::new())
        .await
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            DecisionError::AgentRuntime(AgentRuntimeError::WarmUpDecodeFailed(
                DecodeError::NoKvCacheSlot
            ))
            .to_string()
        )
    );
}
