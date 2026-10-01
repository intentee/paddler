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
    use tokio::sync::watch;
    use tokio_util::sync::CancellationToken;

    use paddler_inference_parameters::inference_parameters::InferenceParameters;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::api_path::ApiPath;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_state_database::memory::Memory;

    use super::get_balancer_applicable_state;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::agent_response_senders::AgentResponseSenders;
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::management_service::app_data::AppData;

    fn build_app_data(
        balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    ) -> Data<AppData> {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            watch::channel(BalancerDesiredState::default());

        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder,
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                Arc::new(AgentControllerPool::default()),
                Duration::from_secs(1),
                10,
            )),
            agent_response_senders: AgentResponseSenders::default(),
            shutdown: CancellationToken::new(),
            state_database: Arc::new(Memory::new(
                balancer_desired_state_notify_tx,
                BalancerDesiredState::default(),
            )),
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn responds_with_stored_agent_desired_state() {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::new(
            BalancerApplicableState::from(BalancerDesiredState::default()),
        ));

        balancer_applicable_state_holder.set_balancer_applicable_state(BalancerApplicableState {
            agent_desired_state: AgentDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                multimodal_projection: AgentDesiredModel::None,
            },
        });

        let app_data = build_app_data(balancer_applicable_state_holder);
        let app = init_service(
            App::new()
                .app_data(app_data)
                .configure(get_balancer_applicable_state),
        )
        .await;
        let request = TestRequest::get()
            .uri(ApiPath::BALANCER_APPLICABLE_STATE)
            .to_request();
        let response = call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let agent_desired_state: AgentDesiredState = read_body_json(response).await;

        assert_eq!(
            agent_desired_state.model,
            AgentDesiredModel::LocalToAgent("model.gguf".to_owned())
        );
    }
}
