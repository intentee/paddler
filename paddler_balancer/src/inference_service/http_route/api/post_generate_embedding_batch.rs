use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::error::ErrorNotImplemented;
use actix_web::error::ErrorServiceUnavailable;
use actix_web::http::header;
use actix_web::post;
use actix_web::rt;
use actix_web::web;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::StreamExt;
use nanoid::nanoid;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::inference_client::message::Message as OutgoingMessage;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::request_params::generate_embedding_batch_params::chunk_evenly_with_cap_error::ChunkEvenlyWithCapError;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;

use crate::cancellation_token_stream_guard::CancellationTokenStreamGuard;
use crate::chunk_forwarding_session_controller::ChunkForwardingSessionController;
use crate::chunk_forwarding_session_controller::identity_transformer::IdentityTransformer;
use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::chunk_forwarding_session_controller::transforms_outgoing_message::TransformsOutgoingMessage;
use crate::controls_session::ControlsSession as _;
use crate::inference_service::app_data::AppData;
use crate::request_from_agent::request_from_agent;

#[derive(Clone)]
struct EmbeddingChunkBodyTransformer;

#[async_trait]
impl TransformsOutgoingMessage for EmbeddingChunkBodyTransformer {
    async fn transform(&self, message: OutgoingMessage) -> Result<Vec<TransformResult>> {
        if let OutgoingMessage::Response(ResponseEnvelope {
            response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            ..
        }) = &message
        {
            return Ok(vec![TransformResult::Discard]);
        }

        let serialized = serde_json::to_string(&message)?;

        Ok(vec![TransformResult::Chunk(serialized)])
    }
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[post("/api/v1/generate_embedding_batch")]
async fn respond(
    app_data: web::Data<AppData>,
    params: web::Json<GenerateEmbeddingBatchParams>,
) -> Result<impl Responder, Error> {
    let balancer_applicable_state_holder = app_data.balancer_applicable_state_holder.clone();
    let Some(agent_desired_state) = balancer_applicable_state_holder.get_agent_desired_state()
    else {
        return Err(ErrorServiceUnavailable(
            "Balancer applicable state is not yet set",
        ));
    };

    if !agent_desired_state.inference_parameters.enable_embeddings {
        return Err(ErrorNotImplemented(
            "Embedding generation is not enabled in the inference parameters",
        ));
    }

    let agent_count = app_data.agent_controller_pool.agents.len();
    let embedding_batch_size = agent_desired_state
        .inference_parameters
        .embedding_batch_size;

    let connection_close = CancellationToken::new();
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel();

    let mut chunk_tasks: JoinSet<()> = JoinSet::new();

    let batches = match params
        .into_inner()
        .chunk_evenly_with_cap(agent_count, embedding_batch_size)
    {
        Ok(batches) => batches,
        Err(ChunkEvenlyWithCapError::ZeroAgentCount) => {
            return Err(ErrorServiceUnavailable("No agents are currently connected"));
        }
        Err(ChunkEvenlyWithCapError::ZeroMaxDocumentsPerChunk) => {
            return Err(ErrorInternalServerError(
                "embedding_batch_size is zero despite validation",
            ));
        }
    };

    for batch in batches {
        let buffered_request_manager_clone = app_data.buffered_request_manager.clone();
        let chunk_tx_clone = chunk_tx.clone();
        let connection_close_clone = connection_close.clone();
        let inference_service_configuration_clone =
            app_data.inference_service_configuration.clone();
        let shutdown_clone = app_data.shutdown.clone();

        chunk_tasks.spawn(async move {
            let request_id: String = nanoid!();
            let session_controller = ChunkForwardingSessionController::new(
                chunk_tx_clone,
                EmbeddingChunkBodyTransformer,
            );

            request_from_agent(
                buffered_request_manager_clone,
                connection_close_clone,
                inference_service_configuration_clone,
                batch,
                request_id,
                session_controller,
                shutdown_clone,
            )
            .await;
        });
    }

    let final_done_chunk_tx = chunk_tx.clone();

    rt::spawn(async move {
        while chunk_tasks.join_next().await.is_some() {}

        let final_request_id: String = nanoid!();
        let mut final_session =
            ChunkForwardingSessionController::new(final_done_chunk_tx, IdentityTransformer::new());

        final_session
            .send_response_safe(OutgoingMessage::Response(ResponseEnvelope {
                generated_by: None,
                request_id: final_request_id,
                response: OutgoingResponse::Embedding(EmbeddingResult::Done),
            }))
            .await;
    });

    drop(chunk_tx);

    let stream =
        CancellationTokenStreamGuard::new(UnboundedReceiverStream::new(chunk_rx), connection_close)
            .filter_map(|transform_result| async move {
                match transform_result {
                    TransformResult::Chunk(content) | TransformResult::Error(content) => {
                        Some(Ok::<_, Error>(Bytes::from(format!("{content}\n"))))
                    }
                    TransformResult::Discard => None,
                }
            });

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(stream))
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::register;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::inference_service::app_data::AppData;
    use crate::inference_service::configuration::Configuration;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::inference_parameters::InferenceParameters;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

    fn agent_with_dropped_receiver(agent_id: &str) -> Arc<AgentController> {
        let (agent_message_tx, agent_message_rx) = mpsc::unbounded_channel();

        drop(agent_message_rx);

        Arc::new(AgentController {
            agent_message_tx,
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            connection_close: CancellationToken::new(),
            desired_slots_total: AtomicValue::<AtomicI32>::new(1),
            download_current: AtomicValue::<AtomicU64>::new(0),
            download_filename: RwLock::new(None),
            download_indeterminate: AtomicValue::<AtomicBool>::new(true),
            download_total: AtomicValue::<AtomicU64>::new(0),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: agent_id.to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(1),
            state_application_status_code: AtomicValue::<AtomicI32>::new(
                AgentStateApplicationStatus::Fresh as i32,
            ),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        })
    }

    fn inference_parameters_with_embeddings(
        enable_embeddings: bool,
        embedding_batch_size: usize,
    ) -> InferenceParameters {
        InferenceParameters {
            embedding_batch_size,
            enable_embeddings,
            ..InferenceParameters::default()
        }
    }

    fn applicable_state(inference_parameters: InferenceParameters) -> BalancerApplicableState {
        BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters,
                model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        }
    }

    fn app_data(
        agent_controller_pool: Arc<AgentControllerPool>,
        balancer_applicable_state: Option<BalancerApplicableState>,
    ) -> AppData {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());

        balancer_applicable_state_holder.set_balancer_applicable_state(balancer_applicable_state);

        AppData {
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool.clone(),
                Duration::from_secs(1),
                10,
            )),
            agent_controller_pool,
            balancer_applicable_state_holder,
            inference_service_configuration: Configuration {
                addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                cors_allowed_hosts: Vec::new(),
                inference_item_timeout: Duration::from_secs(1),
            },
            shutdown: CancellationToken::new(),
        }
    }

    fn single_document_params() -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: vec![EmbeddingInputDocument {
                content: "the quick brown fox".to_owned(),
                id: "doc-1".to_owned(),
            }],
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[actix_web::test]
    async fn responds_service_unavailable_when_balancer_state_is_not_set() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_data(
                    Arc::new(AgentControllerPool::default()),
                    None,
                )))
                .configure(register),
        )
        .await;

        let request = test::TestRequest::post()
            .uri("/api/v1/generate_embedding_batch")
            .set_json(single_document_params())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[actix_web::test]
    async fn responds_service_unavailable_when_no_agents_are_connected() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_data(
                    Arc::new(AgentControllerPool::default()),
                    Some(applicable_state(inference_parameters_with_embeddings(
                        true, 256,
                    ))),
                )))
                .configure(register),
        )
        .await;

        let request = test::TestRequest::post()
            .uri("/api/v1/generate_embedding_batch")
            .set_json(single_document_params())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[actix_web::test]
    async fn responds_internal_server_error_when_embedding_batch_size_is_zero() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        agent_controller_pool
            .register_agent_controller(
                "agent-zero".to_owned(),
                agent_with_dropped_receiver("agent-zero"),
            )
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_data(
                    agent_controller_pool,
                    Some(applicable_state(inference_parameters_with_embeddings(
                        true, 0,
                    ))),
                )))
                .configure(register),
        )
        .await;

        let request = test::TestRequest::post()
            .uri("/api/v1/generate_embedding_batch")
            .set_json(single_document_params())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn streams_error_chunk_when_agent_request_fails() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        agent_controller_pool
            .register_agent_controller(
                "agent-closed".to_owned(),
                agent_with_dropped_receiver("agent-closed"),
            )
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_data(
                    agent_controller_pool,
                    Some(applicable_state(inference_parameters_with_embeddings(
                        true, 256,
                    ))),
                )))
                .configure(register),
        )
        .await;

        let request = test::TestRequest::post()
            .uri("/api/v1/generate_embedding_batch")
            .set_json(single_document_params())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = test::read_body(response).await;
        let body_text = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body_text.contains("Failed to generate response"),
            "streamed body must carry the forwarded agent error chunk, got: {body_text}"
        );
    }
}
