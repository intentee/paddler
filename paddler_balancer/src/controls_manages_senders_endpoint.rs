use std::sync::Arc;

use actix_web::Error;
use actix_web::HttpResponse;
use async_trait::async_trait;
use tokio::time::Duration;
use tokio::time::sleep;

use crate::agent_controller::AgentController;
use crate::agent_controller_pool::AgentControllerPool;
use crate::manages_senders::ManagesSenders;
use crate::manages_senders_controller::ManagesSendersController;

const TIMEOUT: Duration = Duration::from_secs(3);

#[async_trait]
pub trait ControlsManagesSendersEndpoint {
    type SenderCollection: ManagesSenders + Send + Sync + 'static;

    fn get_agent_controller_pool(&self) -> Arc<AgentControllerPool>;

    fn get_agent_id(&self) -> String;

    async fn get_manages_senders_controller(
        &self,
        agent_controller: Arc<AgentController>,
    ) -> anyhow::Result<ManagesSendersController<Self::SenderCollection>>;

    async fn respond(&self) -> Result<HttpResponse, Error> {
        let agent_controller_pool = self.get_agent_controller_pool();
        let agent_id = self.get_agent_id();
        let Some(agent_controller) = agent_controller_pool.get_agent_controller(&agent_id) else {
            return Ok(HttpResponse::NotFound().finish());
        };

        let connection_close = agent_controller.connection_close.clone();

        match self.get_manages_senders_controller(agent_controller).await {
            Ok(mut receive_response_controller) => {
                tokio::select! {
                    () = connection_close.cancelled() => Ok(HttpResponse::BadGateway().finish()),
                    () = sleep(TIMEOUT) => Ok(HttpResponse::GatewayTimeout().finish()),
                    response = receive_response_controller.response_rx.recv() => response.map_or_else(
                        || Ok(HttpResponse::NotFound().finish()),
                        |existing_response| Ok(HttpResponse::Ok().json(existing_response)),
                    ),
                }
            }
            Err(err) => Ok(HttpResponse::InternalServerError().body(format!("{err}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;

    use actix_web::http::StatusCode;
    use async_trait::async_trait;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::ControlsManagesSendersEndpoint;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::manages_senders_controller::ManagesSendersController;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;

    fn registered_agent_id(pool: &AgentControllerPool) -> String {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();
        let agent_id = "agent-test".to_owned();

        pool.register_agent_controller(
            agent_id.clone(),
            Arc::new(AgentController {
                agent_message_tx,
                chat_template_override_sender_collection: Arc::new(
                    ChatTemplateOverrideSenderCollection::default(),
                ),
                connection_close: CancellationToken::new(),
                desired_slots_total: AtomicValue::<AtomicI32>::new(0),
                download_current: AtomicValue::<AtomicU64>::new(0),
                download_filename: RwLock::new(None),
                download_indeterminate: AtomicValue::<AtomicBool>::new(true),
                download_total: AtomicValue::<AtomicU64>::new(0),
                embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
                generate_tokens_sender_collection: Arc::new(
                    GenerateTokensSenderCollection::default(),
                ),
                id: agent_id.clone(),
                issues: RwLock::new(BTreeSet::new()),
                model_metadata_sender_collection: Arc::new(
                    ModelMetadataSenderCollection::default(),
                ),
                model_path: RwLock::new(None),
                name: None,
                newest_update_version: AtomicValue::<AtomicI32>::new(0),
                slots_processing: AtomicValue::<AtomicI32>::new(0),
                slots_total: AtomicValue::<AtomicI32>::new(0),
                state_application_status_code: AtomicValue::<AtomicI32>::new(
                    AgentStateApplicationStatus::Fresh as i32,
                ),
                uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
            }),
        )
        .unwrap();

        agent_id
    }

    struct EndpointWithClosedResponseChannel {
        agent_controller_pool: Arc<AgentControllerPool>,
        agent_id: String,
    }

    #[async_trait]
    impl ControlsManagesSendersEndpoint for EndpointWithClosedResponseChannel {
        type SenderCollection = ModelMetadataSenderCollection;

        fn get_agent_controller_pool(&self) -> Arc<AgentControllerPool> {
            self.agent_controller_pool.clone()
        }

        fn get_agent_id(&self) -> String {
            self.agent_id.clone()
        }

        async fn get_manages_senders_controller(
            &self,
            _agent_controller: Arc<AgentController>,
        ) -> anyhow::Result<ManagesSendersController<Self::SenderCollection>> {
            let response_sender_collection = Arc::new(ModelMetadataSenderCollection::default());
            let (response_tx, response_rx) = mpsc::unbounded_channel();

            drop(response_tx);

            Ok(ManagesSendersController {
                request_id: "closed-request".to_owned(),
                response_rx,
                response_sender_collection,
            })
        }
    }

    #[actix_web::test]
    async fn responds_not_found_when_response_channel_yields_no_value() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let agent_id = registered_agent_id(&agent_controller_pool);

        let endpoint = EndpointWithClosedResponseChannel {
            agent_controller_pool,
            agent_id,
        };

        let response = endpoint.respond().await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
