pub mod app_data;
pub mod arguments_to_tool_call_string;
pub mod assistant_role;
pub mod chat_completion;
pub mod chat_completion_chunk;
pub mod chat_completion_chunk_choice;
pub mod chat_completion_chunk_payload;
pub mod chat_completion_finish_reason;
pub mod chat_completion_tool_call;
pub mod chat_completions_sse_response;
pub mod configuration;
pub mod content_part_event;
pub mod function_call_arguments_delta_event;
pub mod function_call_arguments_done_event;
pub mod http_route;
pub mod open_item;
pub mod openai_api_path;
pub mod openai_chat_completion_tool;
pub mod openai_completion_request_params;
pub mod openai_default_max_tokens;
pub mod openai_error;
pub mod openai_error_type;
pub mod openai_function_definition;
pub mod openai_json_config;
pub mod openai_json_response;
pub mod openai_message;
pub mod openai_non_streaming_response_transformer;
pub mod openai_non_streaming_state;
pub mod openai_reasoning_effort;
pub mod openai_responses_function_call_item;
pub mod openai_responses_function_call_output_item;
pub mod openai_responses_function_output;
pub mod openai_responses_function_output_part;
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
pub mod openai_usage;
pub mod output_item_event;
pub mod response_snapshot_event;
pub mod responses_content_part;
pub mod responses_item_status;
pub mod responses_non_streaming_response_transformer;
pub mod responses_non_streaming_state;
pub mod responses_output_item;
pub mod responses_output_item_kind;
pub mod responses_prepared_request;
pub mod responses_reasoning_part;
pub mod responses_response;
pub mod responses_response_header;
pub mod responses_response_progress;
pub mod responses_response_status;
pub mod responses_sse_response;
pub mod responses_stream_event;
pub mod responses_streaming_response_transformer;
pub mod responses_streaming_state;
pub mod responses_usage;
pub mod stream_options;
pub mod text_delta_event;
pub mod text_done_event;
pub mod timestamp_from;
pub mod try_universal_error_chunk;

use std::sync::Arc;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::compatibility::openai_service::app_data::AppData;
use crate::compatibility::openai_service::http_route::post_chat_completions::post_chat_completions;
use crate::compatibility::openai_service::http_route::post_responses::post_responses;
use crate::create_cors_middleware::create_cors_middleware;
use crate::http_listener::HttpListener;
use crate::http_route::get_health::get_health;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;

pub struct OpenAIService {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub http_listener: HttpListener,
    pub inference_service_configuration: InferenceServiceConfiguration,
}

#[async_trait]
impl Service for OpenAIService {
    fn name(&self) -> &'static str {
        "balancer::compatibility::openai_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let cors_allowed_hosts_arc = Arc::new(
            self.inference_service_configuration
                .cors_allowed_hosts
                .clone(),
        );

        let app_data = Data::new(AppData {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            shutdown: shutdown.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                        .app_data(app_data.clone())
                        .configure(get_health)
                        .configure(post_chat_completions)
                        .configure(post_responses)
                },
                http_listener: self.http_listener,
                service_name,
                worker_count: 16,
            },
        )
        .await
    }
}
