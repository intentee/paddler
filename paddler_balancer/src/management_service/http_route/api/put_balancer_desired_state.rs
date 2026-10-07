use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error::ErrorInternalServerError;
use actix_web::error::ErrorUnprocessableEntity;
use actix_web::web;
use actix_web::web::put;

use paddler_messaging::api_path::ApiPath;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::subscribes_to_updates::SubscribesToUpdates as _;
use paddler_state_database::state_database_error::StateDatabaseError;

use crate::management_service::app_data::AppData;

async fn respond(
    app_data: web::Data<AppData>,
    balancer_desired_state: web::Json<BalancerDesiredState>,
) -> Result<impl Responder, Error> {
    let mut applied_state_rx = app_data
        .balancer_applicable_state_holder
        .subscribe_to_updates();

    app_data
        .state_database
        .store_balancer_desired_state(&balancer_desired_state.into_inner())
        .await
        .map_err(|state_database_error| match state_database_error {
            StateDatabaseError::RequestedStateServesAnotherMode { .. } => {
                ErrorUnprocessableEntity(state_database_error)
            }
            unstorable_state => ErrorInternalServerError(unstorable_state),
        })?;

    applied_state_rx
        .changed()
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
    use actix_web::rt::spawn;
    use actix_web::test::TestRequest;
    use actix_web::test::call_service;
    use actix_web::test::init_service;
    use actix_web::web::Data;
    use serde_json::json;
    use serde_json::to_value;
    use tempfile::TempDir;
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_mode::InferenceMode;
    use paddler_state_database::file::File;
    use paddler_state_database::memory::Memory;
    use paddler_state_database::state_database::StateDatabase;

    use super::put_balancer_desired_state;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;
    use crate::reconciliation_service::ReconciliationService;

    fn build_app_data(state_database: Arc<dyn StateDatabase>) -> Data<AppData> {
        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::new(
                InferenceMode::TextGeneration,
            )),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                BalancerApplicableState::from(BalancerDesiredState::unconfigured(
                    InferenceMode::TextGeneration,
                )),
            )),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration)),
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
    async fn responds_once_the_stored_state_is_applicable() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_rx) = watch::channel(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        );
        let app_data = build_app_data(Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            InferenceMode::TextGeneration,
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        )));
        let reconciliation_shutdown = CancellationToken::new();
        let reconciliation = spawn(
            Box::new(ReconciliationService {
                agent_controller_pool: app_data.agent_controller_pool.clone(),
                balancer_applicable_state_holder: app_data.balancer_applicable_state_holder.clone(),
                balancer_desired_state_rx,
            })
            .run(reconciliation_shutdown.clone()),
        );
        let app = init_service(
            App::new()
                .app_data(app_data.clone())
                .configure(put_balancer_desired_state),
        )
        .await;
        let applied_model = AgentDesiredModel::LocalToAgent("applied-model".to_owned());
        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(BalancerDesiredState {
                model: applied_model.clone(),
                ..BalancerDesiredState::unconfigured(InferenceMode::TextGeneration)
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

        reconciliation_shutdown.cancel();
        reconciliation.await.unwrap().unwrap();
    }

    #[actix_web::test]
    async fn responds_with_bad_request_when_model_runtime_parameters_are_invalid() {
        let (balancer_desired_state_notify_tx, balancer_desired_state_notify_rx) = watch::channel(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        );
        let state_database = Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            InferenceMode::TextGeneration,
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        ));
        let app_data = build_app_data(state_database);
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(put_balancer_desired_state),
        )
        .await;
        let mut invalid_desired_state = to_value(BalancerDesiredState::unconfigured(
            InferenceMode::TextGeneration,
        ))
        .unwrap();

        invalid_desired_state["model_runtime_parameters"]["n_gpu_layers"] = json!(-2);

        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(invalid_desired_state)
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        drop(balancer_desired_state_notify_rx);
    }

    #[actix_web::test]
    async fn responds_with_internal_server_error_when_store_fails() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) = watch::channel(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        );
        let directory_in_place_of_the_state_file =
            TempDir::new().expect("a temporary directory must be creatable");
        let app_data = build_app_data(Arc::new(File::new(
            balancer_desired_state_notify_tx,
            InferenceMode::TextGeneration,
            directory_in_place_of_the_state_file.path().to_path_buf(),
        )));
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(put_balancer_desired_state),
        )
        .await;
        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(BalancerDesiredState::unconfigured(
                InferenceMode::TextGeneration,
            ))
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn responds_with_unprocessable_entity_when_the_state_serves_another_mode() {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) = watch::channel(
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        );
        let app_data = build_app_data(Arc::new(Memory::new(
            balancer_desired_state_notify_tx,
            InferenceMode::TextGeneration,
            BalancerDesiredState::unconfigured(InferenceMode::TextGeneration),
        )));
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(put_balancer_desired_state),
        )
        .await;
        let request = TestRequest::put()
            .uri(ApiPath::BALANCER_DESIRED_STATE)
            .set_json(BalancerDesiredState::unconfigured(
                InferenceMode::Embeddings,
            ))
            .to_request();

        assert_eq!(
            call_service(&app, request).await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
