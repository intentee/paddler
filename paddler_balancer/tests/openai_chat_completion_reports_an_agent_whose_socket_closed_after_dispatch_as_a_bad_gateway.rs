use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use actix_web::App;
use actix_web::http::StatusCode;
use actix_web::test::TestRequest;
use actix_web::test::call_service;
use actix_web::test::init_service;
use actix_web::test::read_body_json;
use actix_web::web::Data;
use parking_lot::RwLock;
use serde_json::Value;
use serde_json::json;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use paddler_balancer::agent_controller::AgentController;
use paddler_balancer::agent_controller_pool::AgentControllerPool;
use paddler_balancer::agent_response_senders::AgentResponseSenders;
use paddler_balancer::balancer_applicable_state::BalancerApplicableState;
use paddler_balancer::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use paddler_balancer::buffered_request_manager::BufferedRequestManager;
use paddler_balancer::compatibility::compatibility_app_data::CompatibilityAppData;
use paddler_balancer::compatibility::openai_service::http_route::post_chat_completions::post_chat_completions;
use paddler_balancer::compatibility::openai_service::openai_api_path::OpenAIApiPath;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::atomic_value::AtomicValue;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::slot_aggregated_status_snapshot::SlotAggregatedStatusSnapshot;

const LONGER_THAN_THE_TEST: Duration = Duration::from_hours(1);

#[actix_web::test]
async fn openai_chat_completion_reports_an_agent_whose_socket_closed_after_dispatch_as_a_bad_gateway()
 {
    let agent_controller_pool = Arc::new(AgentControllerPool::new(InferenceMode::TextGeneration));
    let (agent_message_tx, agent_message_rx) = mpsc::unbounded_channel();

    drop(agent_message_rx);

    let _agent_controller_registration =
        agent_controller_pool.register_agent_controller(Arc::new(AgentController {
            agent_message_tx,
            agent_response_senders: AgentResponseSenders::default(),
            connection_close: CancellationToken::new(),
            id: "closed-agent".to_owned(),
            name: None,
            reported_status: RwLock::new(SlotAggregatedStatusSnapshot {
                status: AgentStatus {
                    runtime: AgentRuntimeStatus::Serving {
                        inference_mode: InferenceMode::TextGeneration,
                        slots_total: 1,
                    },
                    ..AgentStatus::default()
                },
                version: 1,
            }),
            slots_processing: AtomicValue::<AtomicU64>::new(0),
        }));
    let app = init_service(
        App::new()
            .app_data(Data::new(CompatibilityAppData {
                balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::new(
                    BalancerApplicableState::from(BalancerDesiredState::unconfigured(
                        InferenceMode::TextGeneration,
                    )),
                )),
                buffered_request_manager: Arc::new(BufferedRequestManager::new(
                    agent_controller_pool,
                    LONGER_THAN_THE_TEST,
                    1,
                )),
                inference_service_configuration: InferenceServiceConfiguration {
                    addr: ResolvedSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))),
                    cors_allowed_hosts: Vec::new(),
                    inference_item_timeout: LONGER_THAN_THE_TEST,
                },
                shutdown: CancellationToken::new(),
            }))
            .configure(post_chat_completions),
    )
    .await;

    let response = call_service(
        &app,
        TestRequest::post()
            .uri(OpenAIApiPath::CHAT_COMPLETIONS)
            .set_json(json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        read_body_json::<Value, _>(response).await,
        json!({
            "error": {
                "message": "Failed to generate response",
                "type": "server_error",
                "param": null,
                "code": null
            }
        })
    );
}
