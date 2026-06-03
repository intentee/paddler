use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::get;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_desired_state")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let desired_state = app_data
        .state_database
        .read_balancer_desired_state()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(desired_state))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web::Data;
    use tempfile::TempDir;
    use tokio::sync::broadcast;
    use tokio_util::sync::CancellationToken;

    use super::register;
    use crate::agent_desired_model::AgentDesiredModel;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::buffered_request_manager::BufferedRequestManager;
    use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::balancer::management_service::app_data::AppData;
    use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::balancer::state_database::File;
    use crate::balancer::state_database::Memory;
    use crate::balancer::state_database::StateDatabase;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::balancer_desired_state::BalancerDesiredState;
    use crate::inference_parameters::InferenceParameters;

    fn build_app_data(state_database: Arc<dyn StateDatabase>) -> Data<AppData> {
        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
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
            state_database,
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn responds_with_stored_desired_state() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);
        let stored_state = BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        };
        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            stored_state.clone(),
        ));
        let app_data = build_app_data(state_database);
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/api/v1/balancer_desired_state")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let desired_state: BalancerDesiredState = test::read_body_json(response).await;

        assert_eq!(desired_state, stored_state);
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_reading_state_fails() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);
        let temp_dir = TempDir::new().unwrap();
        let state_database = Arc::new(File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        ));
        let app_data = build_app_data(state_database);
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/api/v1/balancer_desired_state")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
