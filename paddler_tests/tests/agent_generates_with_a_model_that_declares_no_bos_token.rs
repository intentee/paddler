#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use hf_hub::Cache;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_model_source::model_source::ModelSource;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::qwen3_desired_state::qwen3_desired_state;
use paddler_tests::start_single_agent_cluster_with_desired_state::start_single_agent_cluster_with_desired_state;
use paddler_tests::write_patched_gguf::write_patched_gguf;

#[tokio::test(flavor = "multi_thread")]
async fn agent_generates_with_a_model_that_declares_no_bos_token() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_without_bos_path = model_directory.path().join("qwen3-without-bos.gguf");

    write_patched_gguf(
        &qwen3_0_6b()
            .reference
            .cached_path(&Cache::from_env())
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_without_bos_path,
        |gguf_context| {
            gguf_context.set_val_str(c"tokenizer.ggml.pre", c"chatglm-bpe");
            gguf_context
                .remove_key("tokenizer.ggml.bos_token_id")
                .expect("Qwen3 must declare a BOS token to remove");
        },
    )
    .expect("the model without a BOS token must be writable");

    let cluster = start_single_agent_cluster_with_desired_state(BalancerDesiredState {
        model: ModelSource::LocalToAgent(model_without_bos_path.display().to_string())
            .into_agent_desired_model(),
        ..qwen3_desired_state()
    })
    .await
    .expect("the cluster must start");

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Name a primary color.".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await
        .expect("the inference request must be accepted");

    assert_eq!(
        collected
            .summary()
            .expect("the generation must finish with a summary")
            .usage
            .completion_tokens(),
        1
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
