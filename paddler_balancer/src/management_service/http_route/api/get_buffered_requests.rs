use actix_web::HttpResponse;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;

use crate::management_service::app_data::AppData;

async fn respond(app_data: web::Data<AppData>) -> HttpResponse {
    HttpResponse::Ok().json(app_data.buffered_request_manager.make_snapshot())
}

pub fn get_buffered_requests(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::BUFFERED_REQUESTS, get().to(respond));
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use actix_web::App;
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::test::read_body_json;
    use actix_web::web::Data;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_state_database::memory::Memory;

    use super::get_buffered_requests;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;

    #[actix_web::test]
    async fn responds_with_current_buffered_request_count() {
        let buffered_request_manager = Arc::new(BufferedRequestManager::new(
            Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration)),
            Duration::from_secs(1),
            10,
        ));

        let _first_buffered_request = buffered_request_manager
            .buffered_request_counter
            .try_admit();
        let _second_buffered_request = buffered_request_manager
            .buffered_request_counter
            .try_admit();

        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) = watch::channel(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        );

        let app_data = Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::new(
                InferenceMode::TextGeneration,
            )),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                BalancerApplicableState::from(BalancerDesiredState::unconfigured(
                    InferenceMode::TextGeneration,
                )),
            )),
            buffered_request_manager,
            agent_response_senders: AgentResponseSenders::default(),
            shutdown: CancellationToken::new(),
            state_database: Arc::new(Memory::new(
                balancer_desired_state_notify_tx,
                InferenceMode::TextGeneration,
                BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
            )),
            statsd_prefix: "paddler".to_owned(),
        });

        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(get_buffered_requests),
        )
        .await;
        let request = TestRequest::get()
            .uri(ApiPath::BUFFERED_REQUESTS)
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let snapshot: BufferedRequestManagerSnapshot = read_body_json(response).await;

        assert_eq!(snapshot.buffered_requests_current, 2);
    }
}
