use std::sync::Arc;

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio::sync::mpsc;

use crate::from_request_params::FromRequestParams;
use crate::slot_aggregated_status::SlotAggregatedStatus;
use crate::slot_guard::SlotGuard;

pub struct ContinueFromConversationHistoryRequest {
    pub generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_tokens_tx: mpsc::UnboundedSender<GeneratedTokenResult>,
    pub params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    pub slot_guard: SlotGuard,
}

impl FromRequestParams for ContinueFromConversationHistoryRequest {
    type RequestParams = ContinueFromConversationHistoryParams<ValidatedParametersSchema>;
    type Response = GeneratedTokenResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_tokens_tx: mpsc::UnboundedSender<Self::Response>,
        generate_tokens_stop_rx: mpsc::UnboundedReceiver<()>,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Self {
        Self {
            generate_tokens_stop_rx,
            generated_tokens_tx,
            params,
            slot_guard: SlotGuard::new(slot_aggregated_status),
        }
    }
}
