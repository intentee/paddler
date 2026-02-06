pub mod app_data;
pub mod configuration;
pub mod http_route;

use std::sync::Arc;

use actix_web::App;
use actix_web::HttpServer;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::sync::broadcast;

use crate::balancer::agent_controller_pool::AgentControllerPool;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::balancer::embedding_sender_collection::EmbeddingSenderCollection;
use crate::balancer::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::balancer::http_route as common_http_route;
use crate::balancer::management_service::app_data::AppData;
use crate::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use crate::balancer::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::balancer::state_database::StateDatabase;
#[cfg(feature = "web_admin_panel")]
use crate::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::create_cors_middleware::create_cors_middleware;
use crate::service::Service;

pub struct ManagementService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub configuration: ManagementServiceConfiguration,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

#[async_trait]
impl Service for ManagementService {
    fn name(&self) -> &'static str {
        "balancer::management_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        #[allow(unused_mut)]
        let mut cors_allowed_hosts = self.configuration.cors_allowed_hosts.clone();

        #[cfg(feature = "web_admin_panel")]
        if let Some(web_admin_panel_config) = &self.web_admin_panel_service_configuration {
            cors_allowed_hosts.push(format!("http://{}", web_admin_panel_config.addr));
        }

        let cors_allowed_hosts_arc = Arc::new(cors_allowed_hosts);

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
            state_database: self.state_database.clone(),
            statsd_prefix: self.statsd_prefix.clone(),
        });

        HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(cors_allowed_hosts_arc.clone()))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::api::get_agents::register)
                .configure(http_route::api::get_agents_stream::register)
                .configure(http_route::api::get_balancer_desired_state::register)
                .configure(http_route::api::get_buffered_requests::register)
                .configure(http_route::api::get_buffered_requests_stream::register)
                .configure(http_route::api::get_chat_template_override::register)
                .configure(http_route::api::get_model_metadata::register)
                .configure(http_route::api::put_balancer_desired_state::register)
                .configure(http_route::api::ws_agent_socket::register)
                .configure(http_route::get_metrics::register)
        })
        .shutdown_signal(async move {
            if let Err(err) = shutdown.recv().await {
                error!("Failed to receive shutdown signal: {err}");
            }
        })
        .bind(self.configuration.addr)
        .expect("Unable to bind server to address")
        .run()
        .await?;

        Ok(())
    }
}
