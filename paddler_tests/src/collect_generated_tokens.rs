use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::inference_client::Response as InferenceResponse;
use paddler_types::streamable_result::StreamableResult as _;

use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::inference_message_stream::InferenceMessageStream;

pub async fn collect_generated_tokens(
    mut stream: InferenceMessageStream,
) -> Result<CollectedGeneratedTokens> {
    let mut text = String::new();
    let mut token_results: Vec<GeneratedTokenResult> = Vec::new();

    while let Some(item) = stream.next().await {
        let message = item.context("inference stream yielded an error")?;

        match message {
            InferenceMessage::Response(envelope) => match envelope.response {
                InferenceResponse::GeneratedToken(token_result) => {
                    if let GeneratedTokenResult::Token(token_text) = &token_result {
                        text.push_str(token_text);
                    }

                    let is_done = token_result.is_done();

                    token_results.push(token_result);

                    if is_done {
                        break;
                    }
                }
                InferenceResponse::Embedding(_) => {
                    return Err(anyhow!(
                        "unexpected embedding response on a token-generation stream"
                    ));
                }
                InferenceResponse::Timeout => {
                    return Err(anyhow!("inference request timed out on balancer"));
                }
                InferenceResponse::TooManyBufferedRequests => {
                    return Err(anyhow!("balancer rejected request: too many buffered"));
                }
            },
            InferenceMessage::Error(error_envelope) => {
                return Err(anyhow!(
                    "inference stream returned JSON-RPC error code {} ({})",
                    error_envelope.error.code,
                    error_envelope.error.description
                ));
            }
        }
    }

    Ok(CollectedGeneratedTokens {
        text,
        token_results,
    })
}
