pub mod app_data;
pub mod arguments_to_tool_call_string;
pub mod chat_completions_sse_response;
pub mod configuration;
pub mod content_part_event;
pub mod function_call_arguments_delta_event;
pub mod function_call_arguments_done_event;
pub mod function_call_item;
pub mod http_route;
pub mod message_item_done;
pub mod open_item;
pub mod openai_chat_completion_function;
pub mod openai_chat_completion_tool;
pub mod openai_completion_request_params;
pub mod openai_error;
pub mod openai_message;
pub mod openai_non_streaming_response_transformer;
pub mod openai_non_streaming_state;
pub mod openai_responses_function_call_item;
pub mod openai_responses_function_call_output_item;
pub mod openai_responses_function_output;
pub mod openai_responses_function_tool;
pub mod openai_responses_input;
pub mod openai_responses_input_content_part;
pub mod openai_responses_input_item;
pub mod openai_responses_message_content;
pub mod openai_responses_message_item;
pub mod openai_responses_reasoning;
pub mod openai_responses_request_params;
pub mod openai_responses_tagged_item;
pub mod openai_responses_text_format;
pub mod openai_responses_text_param;
pub mod openai_responses_tool;
pub mod openai_streaming_response_transformer;
pub mod openai_streaming_state;
pub mod openai_tool_parameters_schema;
pub mod openai_usage_json;
pub mod output_item_event;
pub mod output_text_part;
pub mod reasoning_item_done;
pub mod response_snapshot_event;
pub mod responses_error;
pub mod responses_non_streaming_response_transformer;
pub mod responses_non_streaming_state;
pub mod responses_prepared_request;
pub mod responses_response_builder;
pub mod responses_stream_event;
pub mod responses_streaming_response_transformer;
pub mod responses_streaming_state;
pub mod sse_response_from_agent;
pub mod stream_options;
pub mod text_delta_event;
pub mod text_done_event;
pub mod timestamp_from;
pub mod try_universal_error_chunk;

use std::sync::Arc;

use actix_web::App;
use actix_web::HttpServer;
use actix_web::web::Data;
use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;
use trzcina::ServiceShutdownOptions;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use crate::create_cors_middleware::create_cors_middleware;
use crate::http_route as common_http_route;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::serve_http_until_shutdown::serve_http_until_shutdown;

const HTTP_WORKERS: usize = 16;

pub struct OpenAIService {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub graceful_http_shutdown: bool,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub openai_service_configuration: OpenAIServiceConfiguration,
    pub shutdown_options: ServiceShutdownOptions,
}

#[async_trait]
impl Service for OpenAIService {
    fn name(&self) -> &'static str {
        "balancer::compatibility::openai_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let cors_allowed_hosts = self
            .inference_service_configuration
            .cors_allowed_hosts
            .clone();
        let cors_allowed_hosts_arc = Arc::new(cors_allowed_hosts);

        let app_data = Data::new(AppData {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            shutdown: shutdown.clone(),
        });

        let bind_addr = self.openai_service_configuration.addr;

        let server = HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::post_chat_completions::register)
                .configure(http_route::post_responses::register)
        })
        .workers(HTTP_WORKERS)
        .shutdown_timeout(self.shutdown_options.cooperative_deadline.as_secs())
        .disable_signals()
        .bind(bind_addr)
        .with_context(|| format!("Unable to bind balancer OpenAI-compat service to {bind_addr}"))?
        .run();

        serve_http_until_shutdown(server, shutdown, self.graceful_http_shutdown).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;
    use trzcina::ServiceShutdownOptions;

    use super::OpenAIService;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;

    fn build_service(addr: SocketAddr) -> OpenAIService {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        OpenAIService {
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
                Duration::from_secs(30),
                32,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                cors_allowed_hosts: vec!["http://127.0.0.1:8080".to_owned()],
                inference_item_timeout: Duration::from_secs(30),
            },
            openai_service_configuration: OpenAIServiceConfiguration { addr },
            graceful_http_shutdown: true,
            shutdown_options: ServiceShutdownOptions::default(),
        }
    }

    #[actix_web::test]
    async fn run_returns_error_when_address_is_already_in_use() {
        let occupied_listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
        let occupied_addr = occupied_listener.local_addr().unwrap();

        let service = Box::new(build_service(occupied_addr));
        let result = service.run(CancellationToken::new()).await;

        let error_message = result.unwrap_err().to_string();
        let expected_addr_fragment = occupied_addr.to_string();

        assert!(error_message.contains(&expected_addr_fragment));
    }
}
