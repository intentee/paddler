use std::pin::Pin;
use std::sync::OnceLock;

use futures_util::Stream;
use nanoid::nanoid;
use reqwest::Client;
use tokio_stream::wrappers::UnboundedReceiverStream;
use url::Url;

use paddler_types::inference_client::Message as InferenceMessage;
use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

use crate::format_api_url::format_api_url;
use crate::inference_socket_pool::InferenceSocketPool;
use crate::stream_ndjson::StreamNdjson;
use crate::Result;

pub struct ClientInference<'client> {
    url: &'client Url,
    http_client: &'client Client,
    inference_socket_pool: OnceLock<InferenceSocketPool>,
    inference_socket_pool_size: usize,
}

impl<'client> ClientInference<'client> {
    pub fn new(
        url: &'client Url,
        http_client: &'client Client,
        inference_socket_pool_size: usize,
    ) -> Self {
        Self {
            url,
            http_client,
            inference_socket_pool: OnceLock::new(),
            inference_socket_pool_size,
        }
    }

    fn get_inference_socket_pool(&self) -> &InferenceSocketPool {
        self.inference_socket_pool.get_or_init(|| {
            InferenceSocketPool::new(self.url.clone(), self.inference_socket_pool_size)
        })
    }

    pub async fn continue_from_conversation_history(
        &self,
        params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send + '_>>> {
        let request_id = nanoid!();
        let rx = self
            .get_inference_socket_pool()
            .send_request(request_id, params)
            .await?;

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    pub async fn continue_from_raw_prompt(
        &self,
        params: ContinueFromRawPromptParams,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send + '_>>> {
        let request_id = nanoid!();
        let rx = self
            .get_inference_socket_pool()
            .send_request(request_id, params)
            .await?;

        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    pub async fn generate_embedding_batch(
        &self,
        params: &GenerateEmbeddingBatchParams,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InferenceMessage>> + Send>>> {
        let response = self
            .http_client
            .post(format_api_url(
                self.url.as_str(),
                "/api/v1/generate_embedding_batch",
            ))
            .json(params)
            .send()
            .await?
            .error_for_status()?;

        let stream = StreamNdjson::<InferenceMessage>::from_response(response);
        Ok(Box::pin(stream))
    }
}
