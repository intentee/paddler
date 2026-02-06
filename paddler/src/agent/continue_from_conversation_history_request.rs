use actix::Message;
use anyhow::Result;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;
use crate::generated_token_result::GeneratedTokenResult;
use crate::request_params::ContinueFromConversationHistoryParams;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct ContinueFromConversationHistoryRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
}

impl FromRequestParams for ContinueFromConversationHistoryRequest {
    type RequestParams = ContinueFromConversationHistoryParams<ValidatedParametersSchema>;
    type Response = GeneratedTokenResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_tokens_tx: mpsc::UnboundedSender<Self::Response>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        ContinueFromConversationHistoryRequest {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
        }
    }
}
