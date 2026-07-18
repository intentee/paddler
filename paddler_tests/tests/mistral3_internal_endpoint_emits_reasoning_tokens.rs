#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use anyhow::anyhow;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_tests::ministral_3_cluster_params::Ministral3ClusterParams;
use paddler_tests::start_cluster_with_ministral_3::start_cluster_with_ministral_3;
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread")]
async fn mistral3_internal_endpoint_emits_reasoning_tokens() -> Result<()> {
    let cluster = start_cluster_with_ministral_3(Ministral3ClusterParams::default()).await?;

    let collected = cluster
        .continue_from_conversation_history(
            CancellationToken::new(),
            &ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text(
                        "What is two plus two? Think step by step.".to_owned(),
                    ),
                    role: "user".to_owned(),
                }]),
                enable_thinking: true,
                grammar: None,
                max_tokens: 200,
                parse_tool_calls: false,
                tools: vec![],
            },
        )
        .await?;

    let reasoning_count = collected
        .token_results
        .iter()
        .filter(|result| matches!(result.token_result, GeneratedTokenResult::ReasoningToken(_)))
        .count();

    assert!(
        reasoning_count > 0,
        "Mistral 3: expected at least one reasoning token from a [THINK]-emitting model (got {reasoning_count})"
    );

    let last = collected
        .token_results
        .last()
        .ok_or_else(|| anyhow!("no token results received"))?;
    let GeneratedTokenResult::Done(summary) = &last.token_result else {
        anyhow::bail!("last result was not Done: {last:?}");
    };

    assert!(summary.usage.prompt_tokens > 0);
    assert!(summary.usage.reasoning_tokens > 0);
    assert_eq!(
        summary.usage.completion_tokens(),
        summary.usage.content_tokens
            + summary.usage.reasoning_tokens
            + summary.usage.undeterminable_tokens
    );

    let reasoning_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ReasoningToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();
    let content_stream: String = collected
        .token_results
        .iter()
        .filter_map(|result| match &result.token_result {
            GeneratedTokenResult::ContentToken(piece) => Some(piece.as_str()),
            _ => None,
        })
        .collect();

    for forbidden in ["[THINK]", "[/THINK]"] {
        assert!(
            !reasoning_stream.contains(forbidden),
            "Mistral 3: reasoning stream leaked marker {forbidden:?}; \
             reasoning_stream={reasoning_stream:?}"
        );
        assert!(
            !content_stream.contains(forbidden),
            "Mistral 3: content stream leaked marker {forbidden:?}; \
             content_stream={content_stream:?}"
        );
    }

    cluster.shutdown().await?;

    Ok(())
}
