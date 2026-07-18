pub mod app_data;
pub mod configuration;
pub mod http_route;
pub mod template_data;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;
use crate::web_admin_panel_service::app_data::AppData;
use crate::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;

pub struct WebAdminPanelService {
    pub configuration: WebAdminPanelServiceConfiguration,
}

#[async_trait]
impl Service for WebAdminPanelService {
    fn name(&self) -> &'static str {
        "balancer::web_admin_panel_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let app_data: Data<AppData> = Data::new(AppData {
            template_data: self.configuration.template_data.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .app_data(app_data.clone())
                        .configure(http_route::favicon::register)
                        .configure(http_route::static_files::register)
                        .configure(http_route::home::register)
                },
                bind_addr: self.configuration.addr,
                service_name,
                worker_count: 2,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::time::Duration;

    use anyhow::Result;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;

    use super::WebAdminPanelService;
    use crate::resolved_socket_addr::ResolvedSocketAddr;
    use crate::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
    use crate::web_admin_panel_service::template_data::TemplateData;

    fn build_service(addr: SocketAddr) -> WebAdminPanelService {
        let loopback_addr = ResolvedSocketAddr {
            input_addr: "127.0.0.1:0".to_owned(),
            socket_addr: addr,
        };

        WebAdminPanelService {
            configuration: WebAdminPanelServiceConfiguration {
                addr,
                template_data: TemplateData {
                    buffered_request_timeout: Duration::from_secs(30),
                    compat_openai_addr: None,
                    inference_addr: loopback_addr.clone(),
                    management_addr: loopback_addr,
                    max_buffered_requests: 32,
                    statsd_addr: None,
                    statsd_prefix: "paddler".to_owned(),
                    statsd_reporting_interval: Duration::from_secs(10),
                },
            },
        }
    }

    #[test]
    fn name_identifies_the_web_admin_panel_service() {
        let service = build_service(SocketAddr::from(([127, 0, 0, 1], 0)));

        assert_eq!(service.name(), "balancer::web_admin_panel_service");
    }

    #[actix_web::test]
    async fn run_serves_until_shutdown_is_requested() -> Result<()> {
        let service = Box::new(build_service(SocketAddr::from(([127, 0, 0, 1], 0))));
        let shutdown = CancellationToken::new();
        let requested_shutdown = shutdown.clone();

        let (run_result, ()) =
            tokio::join!(
                service.run(shutdown),
                async move { requested_shutdown.cancel() }
            );

        run_result
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
