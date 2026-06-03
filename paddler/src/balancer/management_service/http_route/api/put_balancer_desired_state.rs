use crate::balancer_desired_state::BalancerDesiredState;
use crate::validates::Validates;
use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorBadRequest;
use actix_web::error::ErrorInternalServerError;
use actix_web::put;
use actix_web::web;

use crate::balancer::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[put("/api/v1/balancer_desired_state")]
async fn respond(
    app_data: web::Data<AppData>,
    balancer_desired_state: web::Json<BalancerDesiredState>,
) -> Result<impl Responder, Error> {
    let balancer_desired_state_inner = balancer_desired_state.into_inner();

    balancer_desired_state_inner
        .inference_parameters
        .clone()
        .validate()
        .map_err(ErrorBadRequest)?;

    app_data
        .state_database
        .store_balancer_desired_state(&balancer_desired_state_inner)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web::Data;
    use tokio::sync::broadcast;
    use tokio_util::sync::CancellationToken;

    use super::register;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::buffered_request_manager::BufferedRequestManager;
    use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::balancer::management_service::app_data::AppData;
    use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::balancer::state_database::Memory;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::balancer_desired_state::BalancerDesiredState;
    use crate::inference_parameters::InferenceParameters;

    fn build_app_data(state_database: Arc<Memory>) -> Data<AppData> {
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
    async fn stores_desired_state_and_responds_with_no_content() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) =
            broadcast::channel(1);
        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            BalancerDesiredState::default(),
        ));
        let app_data = build_app_data(state_database.clone());
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::put()
            .uri("/api/v1/balancer_desired_state")
            .set_json(BalancerDesiredState::default())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        drop(balancer_desired_state_notify_rx);
    }

    #[actix_web::test]
    async fn responds_with_bad_request_when_inference_parameters_are_invalid() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) =
            broadcast::channel(1);
        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            BalancerDesiredState::default(),
        ));
        let app_data = build_app_data(state_database);
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let invalid_desired_state = BalancerDesiredState {
            inference_parameters: InferenceParameters {
                image_resize_to_fit: 0,
                ..InferenceParameters::default()
            },
            ..BalancerDesiredState::default()
        };
        let request = test::TestRequest::put()
            .uri("/api/v1/balancer_desired_state")
            .set_json(invalid_desired_state)
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        drop(balancer_desired_state_notify_rx);
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_store_fails() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) =
            broadcast::channel(1);

        drop(balancer_desired_state_notify_rx);

        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            BalancerDesiredState::default(),
        ));
        let app_data = build_app_data(state_database);
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::put()
            .uri("/api/v1/balancer_desired_state")
            .set_json(BalancerDesiredState::default())
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
