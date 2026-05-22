pub mod app_data;
pub mod configuration;
pub mod http_route;
pub mod template_data;

use actix_web::App;
use actix_web::HttpServer;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;
use trzcina::ServiceShutdownOptions;

use crate::balancer::web_admin_panel_service::app_data::AppData;
use crate::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;

pub struct WebAdminPanelService {
    pub configuration: WebAdminPanelServiceConfiguration,
    pub shutdown_options: ServiceShutdownOptions,
}

#[async_trait]
impl Service for WebAdminPanelService {
    fn name(&self) -> &'static str {
        "balancer::web_admin_panel_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let app_data: Data<AppData> = Data::new(AppData {
            template_data: self.configuration.template_data.clone(),
        });

        #[expect(clippy::expect_used, reason = "server bind failure is unrecoverable")]
        HttpServer::new(move || {
            App::new()
                .app_data(app_data.clone())
                .configure(http_route::favicon::register)
                .configure(http_route::static_files::register)
                .configure(http_route::home::register)
        })
        .shutdown_signal(async move {
            shutdown.cancelled().await;
        })
        .shutdown_timeout(self.shutdown_options.cooperative_deadline.as_secs())
        .disable_signals()
        .bind(self.configuration.addr)
        .expect("Unable to bind server to address")
        .run()
        .await?;

        Ok(())
    }
}
