use paddler_types::generated_token_result::GeneratedTokenResult;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;

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
        Self {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
        }
    }
}
