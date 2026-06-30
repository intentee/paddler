use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_client_tests::inference_client_for::inference_client_for;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_fixture::local_websocket_fixture::LocalWebSocketFixture;
use paddler_test_fixture::web_socket_behavior::WebSocketBehavior;

fn raw_prompt_params() -> ContinueFromRawPromptParams {
    ContinueFromRawPromptParams {
        grammar: None,
        max_tokens: 16,
        raw_prompt: "hi".to_owned(),
    }
}

const fn conversation_history_params()
-> ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    ContinueFromConversationHistoryParams {
        add_generation_prompt: true,
        conversation_history: ConversationHistory::new(Vec::new()),
        enable_thinking: false,
        grammar: None,
        max_tokens: 16,
        parse_tool_calls: false,
        tools: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn raw_prompt_over_socket_reuses_a_pooled_connection() -> Result<()> {
    let fixture = LocalWebSocketFixture::start(WebSocketBehavior::KeepOpen).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let _first = client
        .socket()
        .continue_from_raw_prompt(raw_prompt_params())
        .await?;
    let _second = client
        .socket()
        .continue_from_raw_prompt(raw_prompt_params())
        .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn conversation_history_over_socket_connects() -> Result<()> {
    let fixture = LocalWebSocketFixture::start(WebSocketBehavior::KeepOpen).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let _stream = client
        .socket()
        .continue_from_conversation_history(conversation_history_params())
        .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn raw_prompt_over_socket_reports_a_dropped_connection() -> Result<()> {
    let fixture = LocalWebSocketFixture::start(WebSocketBehavior::CloseAfterAccept).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let mut stream = client
        .socket()
        .continue_from_raw_prompt(raw_prompt_params())
        .await?;
    let outcome = stream.next().await.context("a streamed item")?;

    assert!(outcome.is_err());

    Ok(())
}
