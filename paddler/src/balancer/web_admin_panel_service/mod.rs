pub mod app_data;
pub mod configuration;
pub mod http_route;
pub mod template_data;

use std::net::TcpListener;

use actix_web::App;
use actix_web::HttpServer;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::balancer::web_admin_panel_service::app_data::AppData;
use crate::balancer::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
use crate::service::Service;

pub struct WebAdminPanelService {
    pub configuration: WebAdminPanelServiceConfiguration,
    pub listener: Option<TcpListener>,
}

#[async_trait]
impl Service for WebAdminPanelService {
    fn name(&self) -> &'static str {
        "balancer::web_admin_panel_service"
    }

    async fn run(&mut self, shutdown: CancellationToken) -> Result<()> {
        let app_data: Data<AppData> = Data::new(AppData {
            template_data: self.configuration.template_data.clone(),
        });

        let taken_listener = self.listener.take();
        let configured_addr = self.configuration.addr;

        let bound = HttpServer::new(move || {
            App::new()
                .app_data(app_data.clone())
                .configure(http_route::favicon::register)
                .configure(http_route::static_files::register)
                .configure(http_route::home::register)
        })
        .shutdown_signal(async move {
            shutdown.cancelled().await;
        })
        .disable_signals();

        #[expect(clippy::expect_used, reason = "server bind failure is unrecoverable")]
        let bound = match taken_listener {
            Some(listener) => bound.listen(listener),
            None => bound.bind(configured_addr),
        }
        .expect("Unable to bind/listen server on address");

        bound.run().await?;

        Ok(())
    }
}
