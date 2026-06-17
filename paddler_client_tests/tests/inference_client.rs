use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_client_tests::inference_client_for::inference_client_for;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::inference_client::response::Response;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_test_fixture::http_response_spec::HttpResponseSpec;
use paddler_test_fixture::local_http_fixture::LocalHttpFixture;

fn ndjson(messages: &[InferenceMessage]) -> Result<Vec<u8>> {
    let mut body = String::new();

    for message in messages {
        body.push_str(&serde_json::to_string(message)?);
        body.push('\n');
    }

    Ok(body.into_bytes())
}

fn generated_token_done(request_id: &str) -> InferenceMessage {
    InferenceMessage::Response(ResponseEnvelope {
        generated_by: None,
        request_id: request_id.to_owned(),
        response: Response::GeneratedToken(GeneratedTokenResult::Done(GenerationSummary::default())),
    })
}

fn embedding_done(request_id: &str) -> InferenceMessage {
    InferenceMessage::Response(ResponseEnvelope {
        generated_by: None,
        request_id: request_id.to_owned(),
        response: Response::Embedding(EmbeddingResult::Done),
    })
}

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

fn embedding_batch_params() -> GenerateEmbeddingBatchParams {
    GenerateEmbeddingBatchParams {
        input_batch: vec![EmbeddingInputDocument {
            content: "hi".to_owned(),
            id: "d1".to_owned(),
        }],
        normalization_method: EmbeddingNormalizationMethod::None,
    }
}

#[tokio::test]
async fn health_returns_the_server_body() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(b"OK".to_vec())).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert_eq!(client.health().await?, "OK");

    Ok(())
}

#[tokio::test]
async fn health_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(client.health().await.is_err());

    Ok(())
}

#[tokio::test]
async fn raw_prompt_over_http_streams_server_messages() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(ndjson(&[generated_token_done("x")])?))
            .await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let mut stream = client.http().continue_from_raw_prompt(&raw_prompt_params()).await?;
    let message = stream.next().await.context("a streamed message")??;

    assert!(matches!(message, InferenceMessage::Response(_)));

    Ok(())
}

#[tokio::test]
async fn raw_prompt_over_http_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(client.http().continue_from_raw_prompt(&raw_prompt_params()).await.is_err());

    Ok(())
}

#[tokio::test]
async fn raw_prompt_collected_drains_until_the_terminal_message() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(ndjson(&[generated_token_done("x")])?))
            .await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let collected = client.http().continue_from_raw_prompt_collected(&raw_prompt_params()).await?;

    assert!(collected.text.is_empty());

    Ok(())
}

#[tokio::test]
async fn conversation_history_collected_drains_until_the_terminal_message() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(ndjson(&[generated_token_done("x")])?))
            .await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let collected = client
        .http()
        .continue_from_conversation_history_collected(&conversation_history_params())
        .await?;

    assert!(collected.text.is_empty());

    Ok(())
}

#[tokio::test]
async fn embedding_batch_collected_drains_until_the_terminal_message() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(ndjson(&[embedding_done("x")])?)).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let collected =
        client.http().generate_embedding_batch_collected(&embedding_batch_params()).await?;

    assert!(collected.embeddings.is_empty());

    Ok(())
}

const UNREACHABLE_URL: &str = "http://127.0.0.1:1";

#[tokio::test]
async fn health_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = inference_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.health().await.is_err());

    Ok(())
}

#[tokio::test]
async fn raw_prompt_over_http_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = inference_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.http().continue_from_raw_prompt(&raw_prompt_params()).await.is_err());

    Ok(())
}

#[tokio::test]
async fn raw_prompt_collected_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(client.http().continue_from_raw_prompt_collected(&raw_prompt_params()).await.is_err());

    Ok(())
}

#[tokio::test]
async fn conversation_history_collected_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(
        client
            .http()
            .continue_from_conversation_history_collected(&conversation_history_params())
            .await
            .is_err()
    );

    Ok(())
}

#[tokio::test]
async fn embedding_batch_collected_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(
        client.http().generate_embedding_batch_collected(&embedding_batch_params()).await.is_err()
    );

    Ok(())
}

#[tokio::test]
async fn health_errors_when_the_body_is_truncated() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::truncated_body()).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    assert!(client.health().await.is_err());

    Ok(())
}
