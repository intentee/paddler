use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::management_service::app_data::AppData;

async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let desired_state = app_data
        .state_database
        .read_balancer_desired_state()
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(desired_state))
}

pub fn get_balancer_desired_state(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::BALANCER_DESIRED_STATE, get().to(respond));
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
    use tempfile::TempDir;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_state_database::file::File;
    use paddler_state_database::memory::Memory;
    use paddler_state_database::state_database::StateDatabase;

    use super::get_balancer_desired_state;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;

    fn build_app_data(state_database: Arc<dyn StateDatabase>) -> Data<AppData> {
        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                BalancerApplicableState::from(BalancerDesiredState::default()),
            )),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::from_secs(1),
                10,
            )),
            agent_response_senders: AgentResponseSenders::default(),
            shutdown: CancellationToken::new(),
            state_database,
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn responds_with_stored_desired_state() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let stored_state = BalancerDesiredState {
            model: AgentDesiredModel::Uri("model.gguf".to_owned()),
            ..BalancerDesiredState::default()
        };
        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            stored_state.clone(),
        ));
        let app_data = build_app_data(state_database);
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(get_balancer_desired_state),
        )
        .await;
        let request = TestRequest::get()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let desired_state: BalancerDesiredState = read_body_json(response).await;

        assert_eq!(desired_state, stored_state);
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_reading_state_fails() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());
        let temp_dir = TempDir::new().unwrap();
        let state_database = Arc::new(File::new(
            balancer_desired_state_notify_tx,
            temp_dir.path().to_path_buf(),
        ));
        let app_data = build_app_data(state_database);
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(get_balancer_desired_state),
        )
        .await;
        let request = TestRequest::get()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
