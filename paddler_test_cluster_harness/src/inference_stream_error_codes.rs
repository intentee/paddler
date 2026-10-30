use futures_util::StreamExt as _;

use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::inference_client::message::Message as InferenceMessage;

use crate::cluster_harness_error::ClusterHarnessError;

pub async fn inference_stream_error_codes(
    mut stream: InferenceMessageStream,
) -> Result<Vec<i32>, ClusterHarnessError> {
    let mut error_codes = Vec::new();

    while let Some(message) = stream.next().await {
        if let InferenceMessage::Error(error_envelope) =
            message.map_err(ClusterHarnessError::TokenStreamFailed)?
        {
            error_codes.push(error_envelope.error.code);
        }
    }

    Ok(error_codes)
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::mem::discriminant;

    use futures_util::stream::iter;

    use paddler_client::error::Error as ClientError;
    use paddler_messaging::generated_token_result::GeneratedTokenResult;
    use paddler_messaging::inference_client::message::Message as InferenceMessage;
    use paddler_messaging::inference_client::response::Response as InferenceResponse;
    use paddler_messaging::jsonrpc::error::Error as JsonRpcError;
    use paddler_messaging::jsonrpc::error_envelope::ErrorEnvelope;
    use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;

    use super::inference_stream_error_codes;
    use crate::cluster_harness_error::ClusterHarnessError;

    #[tokio::test]
    async fn collects_the_code_of_every_error_and_skips_the_other_messages() {
        let error_codes = inference_stream_error_codes(Box::pin(iter([
            Ok(InferenceMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: "request-1".to_owned(),
                response: InferenceResponse::GeneratedToken(GeneratedTokenResult::ContentToken(
                    "hi".to_owned(),
                )),
            })),
            Ok(InferenceMessage::Error(ErrorEnvelope {
                request_id: "request-1".to_owned(),
                error: JsonRpcError {
                    code: 503,
                    description: "unavailable".to_owned(),
                },
            })),
        ])))
        .await
        .unwrap();

        assert_eq!(error_codes, vec![503]);
    }

    #[tokio::test]
    async fn reports_a_stream_that_yields_an_error() {
        let stream_error =
            inference_stream_error_codes(Box::pin(iter([Err(ClientError::ConnectionDropped {
                request_id: "request-1".to_owned(),
            })])))
            .await
            .unwrap_err();

        assert_eq!(
            discriminant(&stream_error),
            discriminant(&ClusterHarnessError::TokenStreamFailed(
                ClientError::ConnectionDropped {
                    request_id: String::new(),
                }
            ))
        );
        assert_eq!(
            stream_error.source().map(ToString::to_string),
            Some(
                ClientError::ConnectionDropped {
                    request_id: "request-1".to_owned(),
                }
                .to_string()
            )
        );
    }
}
