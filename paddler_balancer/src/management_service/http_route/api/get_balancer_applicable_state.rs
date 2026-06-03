use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::get;
use actix_web::web;

use crate::management_service::app_data::AppData;

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(respond);
}

#[get("/api/v1/balancer_applicable_state")]
async fn respond(app_data: web::Data<AppData>) -> Result<impl Responder, Error> {
    let applicable_state = app_data
        .balancer_applicable_state_holder
        .get_agent_desired_state();

    Ok(HttpResponse::Ok().json(applicable_state))
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
    use crate::balancer_applicable_state::BalancerApplicableState;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::management_service::app_data::AppData;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::state_database::Memory;
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::agent_desired_state::AgentDesiredState;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;
    use paddler_messaging::inference_parameters::InferenceParameters;

    fn build_app_data(
        balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    ) -> Data<AppData> {
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);

        Data::new(AppData {
            agent_controller_pool: Arc::new(AgentControllerPool::default()),
            balancer_applicable_state_holder,
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
            state_database: Arc::new(Memory::new(
                balancer_desired_state_notify_tx,
                BalancerDesiredState::default(),
            )),
            statsd_prefix: "paddler".to_owned(),
        })
    }

    #[actix_web::test]
    async fn responds_with_stored_agent_desired_state() {
        let balancer_applicable_state_holder = Arc::new(BalancerApplicableStateHolder::default());

        balancer_applicable_state_holder.set_balancer_applicable_state(Some(
            BalancerApplicableState {
                agent_desired_state: AgentDesiredState {
                    chat_template_override: None,
                    inference_parameters: InferenceParameters::default(),
                    model: AgentDesiredModel::LocalToAgent("model.gguf".to_owned()),
                    multimodal_projection: AgentDesiredModel::None,
                },
            },
        ));

        let app_data = build_app_data(balancer_applicable_state_holder);
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/api/v1/balancer_applicable_state")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let agent_desired_state: AgentDesiredState = test::read_body_json(response).await;

        assert_eq!(
            agent_desired_state.model,
            AgentDesiredModel::LocalToAgent("model.gguf".to_owned())
        );
    }

    #[actix_web::test]
    async fn responds_with_json_null_when_no_state_is_set() {
        let app_data = build_app_data(Arc::new(BalancerApplicableStateHolder::default()));
        let app = test::init_service(App::new().app_data(app_data).configure(register)).await;
        let request = test::TestRequest::get()
            .uri("/api/v1/balancer_applicable_state")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::OK);

        let body = test::read_body(response).await;

        assert_eq!(body.as_ref(), b"null");
    }
}
