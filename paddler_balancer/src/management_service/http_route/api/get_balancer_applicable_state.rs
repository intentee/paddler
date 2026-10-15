use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::get;

use paddler_messaging::api_path::ApiPath;

use crate::management_service::app_data::AppData;

async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let applicable_state = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    Ok(HttpResponse::Ok().json(applicable_state))
}

pub fn get_balancer_applicable_state(cfg: &mut web::ServiceConfig) {
    cfg.route(ApiPath::BALANCER_APPLICABLE_STATE, get().to(respond));
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
    use tokio_util::sync::CancellationToken;

    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_state_database::memory::Memory;
    use paddler_state_database::stored_balancer_desired_state::StoredBalancerDesiredState;

    use super::get_balancer_applicable_state;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_desired_state_store::BalancerDesiredStateStore;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;

    #[actix_web::test]
    async fn responds_with_the_agent_desired_state_derived_from_the_desired_state() {
        let balancer_desired_state_store = BalancerDesiredStateStore::load(Arc::new(Memory::new(
            StoredBalancerDesiredState::Stored(BalancerDesiredState {
                model: AgentDesiredModel::Uri("model.gguf".to_owned()),
                ..BalancerDesiredState::default()
            }),
        )))
        .await
        .expect("the desired state store must load");
        let app = init_service(
            App::new()
                .app_data(Data::new(AppData {
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
                }))
                .configure(get_balancer_applicable_state),
        )
        .await;
        let response = call_service(
            &app,
            TestRequest::get()
                .uri(ApiPath::BALANCER_APPLICABLE_STATE)
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        let agent_desired_state: AgentDesiredState = read_body_json(response).await;

        assert_eq!(
            agent_desired_state.model,
            AgentDesiredModel::Uri("model.gguf".to_owned())
        );
    }
}
