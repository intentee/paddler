#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::LogOptions;
use llama_cpp_bindings::send_logs_to_tracing;
use paddler::agent::continue_from_conversation_history_request::ContinueFromConversationHistoryRequest;
use paddler::agent::continuous_batch_scheduler_command::ContinuousBatchSchedulerCommand;
use paddler_model_tests::collect_generated_tokens::collect_generated_tokens;
use paddler_model_tests::device_test;
use paddler_model_tests::load_test_image_as_data_uri::load_test_image_as_data_uri;
use paddler_model_tests::log_generated_response::log_generated_response;
use paddler_model_tests::managed_model::ManagedModel;
use paddler_model_tests::managed_model_params::ManagedModelParams;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use tokio::sync::mpsc;

const QWEN3_5_0_8B_LAYER_COUNT: u32 = 999;

device_test!(
    two_concurrent_multimodal_requests_both_produce_tokens,
    |device| {
        send_logs_to_tracing(LogOptions::default());

        let managed_model = ManagedModel::from_huggingface(ManagedModelParams {
            inference_parameters: device
                .inference_parameters_for_full_offload(QWEN3_5_0_8B_LAYER_COUNT),
            model: HuggingFaceModelReference {
                filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_owned(),
                repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
                revision: "main".to_owned(),
            },
            multimodal_projection: Some(HuggingFaceModelReference {
                filename: "mmproj-F16.gguf".to_owned(),
                repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
                revision: "main".to_owned(),
            }),
            slots: 4,
        })
        .await?;

        let test_image_data_uri = load_test_image_as_data_uri();

        let build_conversation = || {
            ConversationHistory::new(vec![
                ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "You are a helpful assistant. Give engaging, short, precise answers."
                            .to_owned(),
                    ),
                    role: "system".to_owned(),
                },
                ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "Hello! How can I help you today?".to_owned(),
                    ),
                    role: "assistant".to_owned(),
                },
                ConversationMessage {
                    content: ConversationMessageContent::Parts(vec![
                        ConversationMessageContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: test_image_data_uri.clone(),
                            },
                        },
                        ConversationMessageContentPart::Text {
                            text: "Describe what you see in this image.".to_owned(),
                        },
                    ]),
                    role: "user".to_owned(),
                },
            ])
        };

        let mut receivers = Vec::new();
        let mut stop_senders = Vec::new();

        for _ in 0..2u8 {
            let (generated_tokens_tx, generated_tokens_rx) = mpsc::unbounded_channel();
            let (stop_tx, generate_tokens_stop_rx) = mpsc::unbounded_channel::<()>();

            managed_model
                .handle()
                .command_tx
                .send(
                    ContinuousBatchSchedulerCommand::ContinueFromConversationHistory(
                        ContinueFromConversationHistoryRequest {
                            generated_tokens_tx,
                            generate_tokens_stop_rx,
                            params: ContinueFromConversationHistoryParams {
                                add_generation_prompt: true,
                                conversation_history: build_conversation(),
                                enable_thinking: false,
                                grammar: None,
                                max_tokens: 32,
                                tools: vec![],
                            },
                        },
                    ),
                )
                .map_err(|err| anyhow::anyhow!("Failed to send command: {err}"))?;

            receivers.push(generated_tokens_rx);
            stop_senders.push(stop_tx);
        }

        let (results_0, results_1) = tokio::join!(
            collect_generated_tokens(receivers.remove(0)),
            collect_generated_tokens(receivers.remove(0)),
        );

        let results_0 = results_0?;
        let results_1 = results_1?;

        log_generated_response(&results_0);
        log_generated_response(&results_1);

        for (index, results) in [&results_0, &results_1].iter().enumerate() {
            let token_count = results
                .iter()
                .filter(|result| matches!(result, GeneratedTokenResult::Token(_)))
                .count();

            assert!(
                token_count > 0,
                "Concurrent multimodal request {index} should produce at least one token, got: {results:?}"
            );
            assert!(
                !results
                    .iter()
                    .any(|result| matches!(result, GeneratedTokenResult::SamplerError(_))),
                "Concurrent multimodal request {index} should not produce SamplerError, got: {results:?}"
            );
            assert!(
                matches!(results.last(), Some(GeneratedTokenResult::Done)),
                "Concurrent multimodal request {index} should end with Done, got: {results:?}"
            );
        }

        drop(stop_senders);

        managed_model.shutdown()?;

        Ok(())
    }
);
