#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::sync::Arc;

use llama_cpp_bindings::batch_add_error::BatchAddError;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::mtmd::micro_batch_tokens;
use llama_cpp_bindings::mtmd::mtmd_default_marker;
use llama_cpp_bindings_types::BatchCapacity;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::scheduler_message::SchedulerMessage;
use paddler_agent_text_generation::continuous_batch_scheduler::ContinuousBatchScheduler;
use paddler_agent_text_generation::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent_text_generation::continuous_batch_scheduler_params::ContinuousBatchSchedulerParams;
use paddler_agent_text_generation::model_bos_token::ModelBosToken;
use paddler_agent_text_generation::multimodal_prompt_support::MultimodalPromptSupport;
use paddler_agent_text_generation::resident_multimodal_context::ResidentMultimodalContext;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::media_marker::MediaMarker;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_test_cluster_harness::load_test_image_data_uri::load_test_image_data_uri;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_test_cluster_harness::model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::conversation_generation::ConversationGeneration;
use paddler_tests::gbnf_literal::gbnf_literal;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::single_image_question::SingleImageQuestion;

use crate::first_message_content_request_preparer::FirstMessageContentRequestPreparer;

const ANSWER: &str = "two llamas";
const MAX_TOKENS: NonZeroU32 = NonZeroU32::new(16).unwrap();
const SLOTS: u16 = 2;
const TOKENS_THE_BATCH_HOLDS: NonZeroU16 = NonZeroU16::MIN;

fn last_result(
    mut generated_tokens_rx: UnboundedReceiver<GeneratedTokenResult>,
) -> Option<GeneratedTokenResult> {
    let mut last_result = None;

    while let Ok(generated_token_result) = generated_tokens_rx.try_recv() {
        last_result = Some(generated_token_result);
    }

    last_result
}

#[test]
fn a_generation_step_overflowing_its_batch_fails_as_batch_assembly() {
    let runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the vision model must be in the Hugging Face cache"),
        agent_inference_runtime_context(SLOTS),
    )
    .expect("the vision model must load");
    let llama_context = runtime
        .create_text_generation_llama_context(u32::from(SLOTS))
        .expect("the context must be created");
    let sequence_context_size = llama_context.n_ctx_seq();
    let model = &runtime.loaded_llama_model.model;
    let n_batch = runtime.model_runtime_parameters.n_batch;
    let multimodal_context = ResidentMultimodalContext::load(
        &smolvlm2_256m_mmproj()
            .cached_path()
            .expect("the multimodal projection must be in the Hugging Face cache"),
        model,
        runtime
            .inference_runtime_context
            .vram_residencies
            .multimodal_projection
            .try_occupy()
            .expect("the multimodal projection residency must be vacant"),
    )
    .expect("the multimodal projection must load");
    let request_preparer = FirstMessageContentRequestPreparer {
        llama_runtime: &runtime,
        sequence_context_size,
    }
    .into_multimodal_generation_request_preparer(
        MultimodalPromptSupport {
            micro_batch_tokens: micro_batch_tokens(&llama_context, n_batch.tokens()),
            model_bos_token: ModelBosToken::of(model),
            multimodal_context: Arc::new(multimodal_context),
            n_batch: n_batch.tokens_i32(),
            sequence_context_size,
        },
        MediaMarker::new(
            mtmd_default_marker()
                .expect("llama.cpp must report its media marker")
                .to_owned(),
        ),
    );
    let image_data_uri = load_test_image_data_uri().expect("the test image must load");
    let image_question = || {
        ConversationGeneration::new(
            ContinueFromConversationHistoryParams {
                grammar: Some(gbnf_literal(ANSWER)),
                ..SingleImageQuestion {
                    image_data_uri: image_data_uri.clone(),
                    max_tokens: MAX_TOKENS,
                    question: "How many llamas are in the image?",
                }
                .into_conversation_params()
            },
            runtime
                .inference_runtime_context
                .slot_aggregated_status
                .clone(),
        )
    };
    let ConversationGeneration {
        agent_request: first_image_question,
        generate_tokens_stop_tx: _first_generate_tokens_stop_tx,
        generated_tokens_rx: first_generated_tokens_rx,
    } = image_question();
    let ConversationGeneration {
        agent_request: second_image_question,
        generate_tokens_stop_tx: _second_generate_tokens_stop_tx,
        generated_tokens_rx: second_generated_tokens_rx,
    } = image_question();
    let (scheduler_message_tx, scheduler_message_rx) = unbounded_channel();

    for agent_request in [first_image_question, second_image_question] {
        scheduler_message_tx
            .send(SchedulerMessage::Command(
                request_preparer
                    .prepare_conversation_history(agent_request)
                    .expect("the image question must be prepared"),
            ))
            .expect("the scheduler must accept the image question");
    }
    scheduler_message_tx
        .send(SchedulerMessage::Shutdown)
        .expect("the scheduler must accept the shutdown");

    ContinuousBatchScheduler::new(ContinuousBatchSchedulerParams {
        agent_shutdown: CancellationToken::new(),
        batch: LlamaBatch::new(
            BatchCapacity::from(TOKENS_THE_BATCH_HOLDS),
            i32::from(SLOTS),
        ),
        llama_context,
        scheduler_context: ContinuousBatchSchedulerContext {
            agent_name: None,
            desired_slots_total: SLOTS,
            model,
            n_batch,
            sequence_context_size,
        },
        scheduler_message_rx,
    })
    .run();

    let batch_assembly_failure = GeneratedTokenResult::BatchAssemblyFailed(
        TextGenerationError::BatchAssemblyFailed(BatchAddError::InsufficientSpace(usize::from(
            TOKENS_THE_BATCH_HOLDS.get(),
        )))
        .to_string(),
    );

    assert_eq!(
        [
            last_result(first_generated_tokens_rx),
            last_result(second_generated_tokens_rx),
        ],
        [
            Some(batch_assembly_failure.clone()),
            Some(batch_assembly_failure)
        ]
    );
}
