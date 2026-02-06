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

use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::http_route as common_http_route;
use crate::balancer::inference_service::app_data::AppData;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use crate::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::create_cors_middleware::create_cors_middleware;
use crate::service::Service;

pub struct InferenceService {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: InferenceServiceConfiguration,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

#[async_trait]
impl Service for InferenceService {
    fn name(&self) -> &'static str {
        "balancer::inference_service"
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
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.configuration.clone(),
        });

        HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(cors_allowed_hosts_arc.clone()))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::api::post_continue_from_conversation_history::register)
                .configure(http_route::api::post_continue_from_raw_prompt::register)
                .configure(http_route::api::post_generate_embedding_batch::register)
                .configure(http_route::api::ws_inference_socket::register)
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
