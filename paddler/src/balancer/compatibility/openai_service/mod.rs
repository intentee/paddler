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
use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use crate::balancer::http_route as common_http_route;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::create_cors_middleware::create_cors_middleware;
use crate::service::Service;

pub struct OpenAIService {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub openai_service_configuration: OpenAIServiceConfiguration,
}

#[async_trait]
impl Service for OpenAIService {
    fn name(&self) -> &'static str {
        "balancer::compatibility::openai_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let cors_allowed_hosts = self
            .inference_service_configuration
            .cors_allowed_hosts
            .clone();
        let cors_allowed_hosts_arc = Arc::new(cors_allowed_hosts);

        let app_data = Data::new(AppData {
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
        });

        HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(cors_allowed_hosts_arc.clone()))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::post_chat_completions::register)
        })
        .shutdown_signal(async move {
            if let Err(err) = shutdown.recv().await {
                error!("Failed to receive shutdown signal: {err}");
            }
        })
        .bind(self.openai_service_configuration.addr)
        .expect("Unable to bind server to address")
        .run()
        .await?;

        Ok(())
    }
}
