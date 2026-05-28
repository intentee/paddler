use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use futures_util::StreamExt as _;
use paddler::balancer::inference_client::Message as InferenceMessage;
use paddler::balancer::inference_client::Response as InferenceResponse;
use paddler::streamable_result::StreamableResult as _;

use crate::collected_generated_tokens::CollectedGeneratedTokens;
use crate::inference_message_stream::InferenceMessageStream;
use crate::token_result_with_producer::TokenResultWithProducer;

pub async fn collect_generated_tokens(
    mut stream: InferenceMessageStream,
) -> Result<CollectedGeneratedTokens> {
    let mut text = String::new();
    let mut token_results: Vec<TokenResultWithProducer> = Vec::new();

    while let Some(item) = stream.next().await {
        let message = item.context("inference stream yielded an error")?;

        match message {
            InferenceMessage::Response(envelope) => {
                let generated_by = envelope.generated_by.clone();

                match envelope.response {
                    InferenceResponse::GeneratedToken(token_result) => {
                        if let Some(token_text) = token_result.token_text() {
                            text.push_str(token_text);
                        }

                        let is_done = token_result.is_done();

                        token_results.push(TokenResultWithProducer {
                            token_result,
                            generated_by,
                        });

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
                }
            }
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
