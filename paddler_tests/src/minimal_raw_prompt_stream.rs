use std::num::NonZeroU32;

use tokio_util::sync::CancellationToken;

use paddler_client::client_inference::ClientInference;
use paddler_client::error::Result;
use paddler_client::inference_message_stream::InferenceMessageStream;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

pub async fn minimal_raw_prompt_stream(
    client_inference: &ClientInference,
) -> Result<InferenceMessageStream> {
    client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "open the inference socket".to_owned(),
            },
        )
        .await
}
