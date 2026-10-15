use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::web;
use actix_web::web::put;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::management_service::app_data::AppData;

async fn respond(
    app_data: web::Data<AppData>,
    balancer_desired_state: web::Json<BalancerDesiredState>,
) -> Result<impl Responder, Error> {
    app_data
        .balancer_desired_state_store
        .replace(balancer_desired_state.into_inner())
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::NoContent().finish())
}

pub fn put_balancer_desired_state(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::BALANCER_DESIRED_STATE, put().to(respond));
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
    use actix_web::web::Data;
    use serde_json::json;
    use serde_json::to_value;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_state_database::file::File;
    use paddler_state_database::memory::Memory;
    use paddler_state_database::state_database::StateDatabase;
    use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;

    use super::put_balancer_desired_state;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_desired_state_store::BalancerDesiredStateStore;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;

    async fn build_app_data(state_database: Arc<dyn StateDatabase>) -> Data<AppData> {
        let balancer_desired_state_store = BalancerDesiredStateStore::load(state_database)
            .await
            .expect("the desired state store must load");

        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder: Arc::new(
                balancer_desired_state_store.applicable_state_holder(),
            ),
            balancer_desired_state_store: Arc::new(balancer_desired_state_store),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::from_secs(1),
                10,
            )),
            agent_response_senders: AgentResponseSenders::default(),
            shutdown: CancellationToken::new(),
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn applies_the_desired_state_before_responding() {
        let app_data = build_app_data(Arc::new(Memory::new(
            StoredBalancerDesiredState::NothingStored,
        )))
        .await;
        let app = init_service(
            App::new()
                .app_data(app_data.clone())
                .configure(put_balancer_desired_state),
        )
        .await;
        let applied_model = AgentDesiredModel::Uri("applied-model".to_owned());
        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(BalancerDesiredState {
                model: applied_model.clone(),
                ..BalancerDesiredState::default()
            })
            .to_request();

        assert_eq!(
            call_service(&app, request).await.status(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            app_data
                .balancer_applicable_state_holder
                .get_agent_desired_state()
                .model,
            applied_model
        );
    }

    #[actix_web::test]
    async fn responds_with_bad_request_when_model_runtime_parameters_are_invalid() {
        let app = init_service(
            App::new()
                .app_data(
                    build_app_data(Arc::new(Memory::new(
                        StoredBalancerDesiredState::NothingStored,
                    )))
                    .await,
                )
                .configure(put_balancer_desired_state),
        )
        .await;
        let mut invalid_desired_state = to_value(BalancerDesiredState::default()).unwrap();

        invalid_desired_state["model_runtime_parameters"]["n_gpu_layers"] = json!(-2);

        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(invalid_desired_state)
            .to_request();

        assert_eq!(
            call_service(&app, request).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_store_fails() {
        let directory_in_place_of_the_state_file =
            TempDir::new().expect("a temporary directory must be creatable");
        let app = init_service(
            App::new()
                .app_data(
                    build_app_data(Arc::new(File::new(
                        directory_in_place_of_the_state_file
                            .path()
                            .join("state.json"),
                    )))
                    .await,
                )
                .configure(put_balancer_desired_state),
        )
        .await;
        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(BalancerDesiredState {
                model: AgentDesiredModel::Uri("unstorable-model".to_owned()),
                ..BalancerDesiredState::default()
            })
            .to_request();

        directory_in_place_of_the_state_file
            .close()
            .expect("the temporary directory must be removable");

        assert_eq!(
            call_service(&app, request).await.status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
