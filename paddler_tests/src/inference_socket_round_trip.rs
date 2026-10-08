use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use tokio_util::sync::CancellationToken;

use paddler_client::client_inference::ClientInference;
use paddler_messaging::inference_client::message::Message;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

pub async fn inference_socket_round_trip(client_inference: &ClientInference) -> Result<Message> {
    let first_message = client_inference
        .continue_from_raw_prompt(
            CancellationToken::new(),
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "open the inference socket".to_owned(),
            },
        )
        .await?
        .next()
        .await
        .context("the inference socket must answer the request")??;

    Ok(first_message)
}
