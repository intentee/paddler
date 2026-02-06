use actix::Message;
use anyhow::Result;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;
use crate::generated_token_result::GeneratedTokenResult;
use crate::request_params::ContinueFromRawPromptParams;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct ContinueFromRawPromptRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub params: ContinueFromRawPromptParams,
}

impl FromRequestParams for ContinueFromRawPromptRequest {
    type RequestParams = ContinueFromRawPromptParams;
    type Response = GeneratedTokenResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_tokens_tx: mpsc::UnboundedSender<Self::Response>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        ContinueFromRawPromptRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
        }
    }
}
