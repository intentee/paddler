pub mod app_data;
pub mod configuration;
pub mod http_route;
pub mod template_data;

use actix_web::App;
use actix_web::HttpServer;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use log::error;
use tokio::sync::broadcast;

use crate::balancer::web_admin_panel_service::app_data::AppData;
use crate::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use crate::service::Service;

pub struct WebAdminPanelService {
    pub configuration: WebAdminPanelServiceConfiguration,
}

#[async_trait]
impl Service for WebAdminPanelService {
    fn name(&self) -> &'static str {
        "balancer::web_admin_panel_service"
    }

    async fn run(&mut self, mut shutdown: broadcast::Receiver<()>) -> Result<()> {
        let app_data: Data<AppData> = Data::new(AppData {
            template_data: self.configuration.template_data.clone(),
        });

        HttpServer::new(move || {
            App::new()
                .app_data(app_data.clone())
                .configure(http_route::favicon::register)
                .configure(http_route::static_files::register)
                .configure(http_route::home::register)
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
