use actix::Message;
use anyhow::Result;
use tokio::sync::mpsc;

use crate::agent::from_request_params::FromRequestParams;
use crate::embedding_result::EmbeddingResult;
use crate::request_params::GenerateEmbeddingBatchParams;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct GenerateEmbeddingBatchRequest {
    pub generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    pub generated_embedding_tx: mpsc::UnboundedSender<EmbeddingResult>,
    pub params: GenerateEmbeddingBatchParams,
}

impl FromRequestParams for GenerateEmbeddingBatchRequest {
    type RequestParams = GenerateEmbeddingBatchParams;
    type Response = EmbeddingResult;

    fn from_request_params(
        params: Self::RequestParams,
        generated_embedding_tx: mpsc::UnboundedSender<Self::Response>,
        generate_embedding_stop_rx: mpsc::UnboundedReceiver<()>,
    ) -> Self {
        GenerateEmbeddingBatchRequest {
            generate_embedding_stop_rx,
            generated_embedding_tx,
            params,
        }
    }
}
