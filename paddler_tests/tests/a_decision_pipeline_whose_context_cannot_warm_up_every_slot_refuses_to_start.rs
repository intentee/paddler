#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;
use std::num::NonZeroU32;

use llama_cpp_context::context_error::ContextError;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const SLOTS: NonZeroU16 = NonZeroU16::new(200).unwrap();
const CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT: NonZeroU32 = NonZeroU32::new(256).unwrap();

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_pipeline_whose_context_cannot_warm_up_every_slot_refuses_to_start() {
    let LlamaRuntime {
        inference_runtime_context,
        loaded_llama_model,
        model_runtime_parameters,
    } = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS.get()),
    )
    .expect("the kev model must load");

    assert!(matches!(
        DecisionPipeline {
            decision_slots: SLOTS,
            inference_runtime_context,
            loaded_llama_model,
            model_runtime_parameters: ModelRuntimeParameters {
                context_size: CACHE_CELLS_FEWER_THAN_TWO_PER_SLOT,
                ..model_runtime_parameters
            },
        }
        .spawn(&CancellationToken::new())
        .await,
        Err(DecisionError::AgentRuntime(
            AgentRuntimeError::WarmUpDecodeFailed(ContextError::DecodeNoKvCacheSlot)
        ))
    ));
}
