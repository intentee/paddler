#![cfg(all(
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms",
    feature = "cuda"
))]

use std::fs;
use std::os::unix::process::ExitStatusExt as _;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::StreamExt;
use nix::libc;
use paddler_integration_tests::managed_cluster::ManagedCluster;
use paddler_integration_tests::managed_cluster_params::ManagedClusterParams;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::conversation_history::ConversationHistory;
use paddler_types::conversation_message::ConversationMessage;
use paddler_types::conversation_message_content::ConversationMessageContent;
use paddler_types::conversation_message_content_part::ConversationMessageContentPart;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::image_url::ImageUrl;
use paddler_types::inference_client::Message;
use paddler_types::inference_client::Response;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use serial_test::file_serial;

const QWEN25_VL_3B_LAYER_COUNT: u32 = 999;
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(60);

fn qwen25vl_model() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "Qwen2.5-VL-3B-Instruct-Q4_K_M.gguf".to_owned(),
        repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
}

fn qwen25vl_mmproj() -> AgentDesiredModel {
    AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
        filename: "mmproj-Qwen2.5-VL-3B-Instruct-Q8_0.gguf".to_owned(),
        repo_id: "ggml-org/Qwen2.5-VL-3B-Instruct-GGUF".to_owned(),
        revision: "main".to_owned(),
    })
}

fn load_test_image_as_data_uri() -> Result<String> {
    let image_bytes = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/llamas.jpg"
    ))
    .context("failed to read test fixture llamas.jpg")?;

    let encoded = BASE64_STANDARD.encode(&image_bytes);

    Ok(format!("data:image/jpeg;base64,{encoded}"))
}

#[tokio::test]
#[file_serial]
async fn test_cuda_agent_exits_cleanly_on_sigterm_during_multimodal_inference() -> Result<()> {
    let mut cluster = ManagedCluster::spawn(ManagedClusterParams {
        agent_slots: 4,
        desired_state: BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: QWEN25_VL_3B_LAYER_COUNT,
                ..InferenceParameters::default()
            },
            model: qwen25vl_model(),
            multimodal_projection: qwen25vl_mmproj(),
            use_chat_template_override: false,
        },
        ..ManagedClusterParams::default()
    })
    .await?;

    let test_image_data_uri = load_test_image_as_data_uri()?;

    let mut stream = cluster
        .balancer
        .client()
        .inference()
        .continue_from_conversation_history(ContinueFromConversationHistoryParams::<
            ValidatedParametersSchema,
        > {
            add_generation_prompt: true,
            conversation_history: ConversationHistory::new(vec![ConversationMessage {
                content: ConversationMessageContent::Parts(vec![
                    ConversationMessageContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: test_image_data_uri,
                        },
                    },
                    ConversationMessageContentPart::Text {
                        text: "Describe this image in great detail.".to_owned(),
                    },
                ]),
                role: "user".to_owned(),
            }]),
            enable_thinking: false,
            grammar: None,
            max_tokens: 1000,
            tools: vec![],
        })
        .await?;

    let mut received_token = false;

    while let Some(message) = stream.next().await {
        if let Ok(Message::Response(envelope)) = message
            && let Response::GeneratedToken(GeneratedTokenResult::Token(_)) = envelope.response
        {
            received_token = true;
            break;
        }
    }

    assert!(
        received_token,
        "agent never produced a token before SIGTERM"
    );

    let exit_status = cluster
        .agent
        .sigterm_and_wait_for_exit(SHUTDOWN_DEADLINE)
        .await?;

    assert_ne!(
        exit_status.signal(),
        Some(libc::SIGABRT),
        "agent process aborted (SIGABRT) during shutdown — CUDA cleanup failed; full status: {exit_status:?}"
    );

    if exit_status.code() != Some(0) && exit_status.signal() != Some(libc::SIGTERM) {
        return Err(anyhow!("agent did not exit cleanly: {exit_status:?}"));
    }

    Ok(())
}
