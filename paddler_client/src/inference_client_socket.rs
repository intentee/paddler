use nanoid::nanoid;
use paddler_messaging::inference_server::message::Message as InferenceServerMessage;
use paddler_messaging::inference_server::request::Request as InferenceServerRequest;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::error::Result;
use crate::inference_message_stream::InferenceMessageStream;
use crate::inference_socket::pool::Pool;

pub struct InferenceClientSocket<'client> {
    pool: &'client Pool,
}

impl<'client> InferenceClientSocket<'client> {
    #[must_use]
    pub const fn new(pool: &'client Pool) -> Self {
        Self { pool }
    }

    pub async fn continue_from_conversation_history(
        &self,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request: InferenceServerRequest::ContinueFromConversationHistory(params),
            });
        let receiver = self.pool.send_request(request_id, message).await?;

        Ok(Box::pin(UnboundedReceiverStream::new(receiver)))
    }

    pub async fn continue_from_raw_prompt(
        &self,
        params: ContinueFromRawPromptParams,
    ) -> Result<InferenceMessageStream> {
        let request_id = nanoid!();
        let message: InferenceServerMessage<ValidatedParametersSchema> =
            InferenceServerMessage::Request(RequestEnvelope {
                id: request_id.clone(),
                request: InferenceServerRequest::ContinueFromRawPrompt(params),
            });
        let receiver = self.pool.send_request(request_id, message).await?;

        Ok(Box::pin(UnboundedReceiverStream::new(receiver)))
    }
}
