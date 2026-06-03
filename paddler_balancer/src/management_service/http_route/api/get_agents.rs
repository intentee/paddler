use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;

use crate::management_service::app_data::AppData;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/agents")]
async fn respond(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(
        app_data
            .agent_controller_pool
            .make_snapshot()
            .map_err(ErrorInternalServerError)?,
    ))
}

#[cfg(test)]
mod tests {
    use parking_lot::RwLock;
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web::Data;
    use tokio::sync::broadcast;
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::register;
    use crate::agent_controller::AgentController;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::management_service::app_data::AppData;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::state_database::Memory;
    use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
    use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
    use paddler_messaging::atomic_value::AtomicValue;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    fn agent_controller_with_status_code(status_code: i32) -> Arc<AgentController> {
        let (agent_message_tx, _agent_message_rx) = mpsc::unbounded_channel();

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
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            id: "agent-test".to_owned(),
            issues: RwLock::new(BTreeSet::new()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            model_path: RwLock::new(None),
            name: None,
            newest_update_version: AtomicValue::<AtomicI32>::new(0),
            slots_processing: AtomicValue::<AtomicI32>::new(0),
            slots_total: AtomicValue::<AtomicI32>::new(0),
            state_application_status_code: AtomicValue::<AtomicI32>::new(status_code),
            uses_chat_template_override: AtomicValue::<AtomicBool>::new(false),
        })
    }

    fn app_data_with_pool(agent_controller_pool: Arc<AgentControllerPool>) -> Data<AppData> {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);

        Data::new(AppData {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
                Duration::from_secs(1),
                10,
            )),
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            shutdown: CancellationToken::new(),
            state_database: Arc::new(Memory::new(
                balancer_desired_state_notify_tx,
                BalancerDesiredState::default(),
            )),
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn responds_with_registered_agents_snapshot() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        agent_controller_pool
            .register_agent_controller(
                "agent-test".to_owned(),
                agent_controller_with_status_code(AgentStateApplicationStatus::Fresh as i32),
            )
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_data_with_pool(agent_controller_pool))
                .configure(register),
        )
        .await;
        let request = test::TestRequest::get().uri("/api/v1/agents").to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let snapshot: AgentControllerPoolSnapshot = test::read_body_json(response).await;

        assert_eq!(snapshot.agents.len(), 1);
        assert_eq!(snapshot.agents[0].id, "agent-test");
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_snapshot_fails() {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        agent_controller_pool
            .register_agent_controller(
                "agent-test".to_owned(),
                agent_controller_with_status_code(99),
            )
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_data_with_pool(agent_controller_pool))
                .configure(register),
        )
        .await;
        let request = test::TestRequest::get().uri("/api/v1/agents").to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
