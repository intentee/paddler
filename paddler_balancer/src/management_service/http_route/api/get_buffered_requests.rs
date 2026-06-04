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

#[get("/api/v1/buffered_requests")]
async fn respond(app_data: web::Data<AppData>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().json(
        app_data
            .buffered_request_manager
            .make_snapshot()
            .map_err(ErrorInternalServerError)?,
    ))
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
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::management_service::app_data::AppData;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::state_database::memory::Memory;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;

    #[actix_web::test]
    async fn responds_with_current_buffered_request_count() {
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            Arc::new(AgentControllerPool::default()),
            Duration::from_secs(1),
            10,
        ));

        buffered_request_manager
            .buffered_request_counter
            .increment();
        buffered_request_manager
            .buffered_request_counter
            .increment();

        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);

        let app_data = Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager,
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
        });

        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/api/v1/buffered_requests")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let snapshot: BufferedRequestManagerSnapshot = test::read_body_json(response).await;

        assert_eq!(snapshot.buffered_requests_current, 2);
    }
}
