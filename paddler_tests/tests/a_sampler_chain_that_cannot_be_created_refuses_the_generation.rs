#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::prepares_scheduler_command::PreparesSchedulerCommand as _;
use paddler_agent_text_generation::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent_text_generation::json_tools_serializer::JsonToolsSerializer;
use paddler_agent_text_generation::text_generation_request::TextGenerationRequest;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::first_message_content_chat_template::first_message_content_chat_template;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::raw_prompt_generation::RawPromptGeneration;
use paddler_tests::sampler_chain_factory_that_fails::SamplerChainFactoryThatFails;
use paddler_tests::text_only_generation_parts::TextOnlyGenerationParts;

#[test]
fn a_sampler_chain_that_cannot_be_created_refuses_the_generation() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b_target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev model must load");
    let preparer = runtime.text_only_generation_request_preparer(
        TextOnlyGenerationParts {
            chat_template_renderer:
                ChatTemplateRenderer::new(first_message_content_chat_template())
                    .expect("the chat template must compile"),
            sequence_context_size: runtime.model_runtime_parameters.context_size.get(),
            streaming_markers: runtime
                .loaded_llama_model
                .model
                .streaming_markers()
                .expect("the streaming markers must be detected"),
        },
        SamplerChainFactoryThatFails,
        JsonToolsSerializer,
    );
    let RawPromptGeneration {
        agent_request,
        generate_tokens_stop_tx: _generate_tokens_stop_tx,
        mut generated_tokens_rx,
    } = RawPromptGeneration::new(
        "Paddler",
        runtime.inference_runtime_context.slot_aggregated_status,
    );

    let scheduler_command = preparer.prepare_scheduler_command(
        Some("test-agent"),
        TextGenerationRequest::ContinueFromRawPrompt(agent_request),
    );

    assert!(scheduler_command.is_none());
    assert_eq!(
        generated_tokens_rx.try_recv(),
        Ok(GeneratedTokenResult::SamplerChainCreationFailed(
            "test-agent: failed to create sampler chain: chain sampler could not be initialized"
                .to_owned()
        ))
    );
}
