pub mod app_data;
pub mod configuration;
pub mod http_route;

use std::net::SocketAddr;
use std::sync::Arc;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::cors_allowed_hosts_with_web_admin_panel::cors_allowed_hosts_with_web_admin_panel;
use crate::create_cors_middleware::create_cors_middleware;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::http_listener::HttpListener;
use crate::http_route as common_http_route;
use crate::management_service::app_data::AppData;
use crate::management_service::configuration::Configuration as ManagementServiceConfiguration;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;
use crate::state_database::StateDatabase;

pub struct ManagementService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub configuration: ManagementServiceConfiguration,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub http_listener: HttpListener,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
    pub web_admin_panel_addr: Option<SocketAddr>,
}

#[async_trait]
impl Service for ManagementService {
    fn name(&self) -> &'static str {
        "balancer::management_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let cors_allowed_hosts_arc = cors_allowed_hosts_with_web_admin_panel(
            &self.configuration.cors_allowed_hosts,
            self.web_admin_panel_addr,
        );

        let app_data = Data::new(AppData {
            agent_controller_pool: self.agent_controller_pool.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            chat_template_override_sender_collection: self
                .chat_template_override_sender_collection
                .clone(),
            embedding_sender_collection: self.embedding_sender_collection.clone(),
            generate_tokens_sender_collection: self.generate_tokens_sender_collection.clone(),
            model_metadata_sender_collection: self.model_metadata_sender_collection.clone(),
            shutdown: shutdown.clone(),
            state_database: self.state_database.clone(),
            statsd_prefix: self.statsd_prefix.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                        .app_data(app_data.clone())
                        .configure(common_http_route::get_health::register)
                        .configure(http_route::api::get_agents::register)
                        .configure(http_route::api::get_agents_stream::register)
                        .configure(http_route::api::get_balancer_applicable_state::register)
                        .configure(http_route::api::get_balancer_desired_state::register)
                        .configure(http_route::api::get_buffered_requests::register)
                        .configure(http_route::api::get_buffered_requests_stream::register)
                        .configure(http_route::api::get_chat_template_override::register)
                        .configure(http_route::api::get_model_metadata::register)
                        .configure(http_route::api::put_balancer_desired_state::register)
                        .configure(http_route::api::ws_agent_socket::register)
                        .configure(http_route::get_metrics::register)
                },
                http_listener: self.http_listener,
                service_name,
                worker_count: 2,
            },
        )
        .await
    }
}
