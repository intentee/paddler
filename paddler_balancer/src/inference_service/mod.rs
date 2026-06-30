pub mod app_data;
pub mod configuration;
pub mod http_route;

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

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::create_cors_middleware::create_cors_middleware;
use crate::http_route as common_http_route;
use crate::inference_service::app_data::AppData;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
#[cfg(feature = "web_admin_panel")]
use crate::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;

const HTTP_WORKERS: usize = 16;

pub struct InferenceService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: InferenceServiceConfiguration,
    pub shutdown_options: ServiceShutdownOptions,
    #[cfg(feature = "web_admin_panel")]
    pub web_admin_panel_service_configuration: Option<WebAdminPanelServiceConfiguration>,
}

#[async_trait]
impl Service for InferenceService {
    fn name(&self) -> &'static str {
        "balancer::inference_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let web_admin_panel_cors_allowed_hosts: Vec<String> = {
            #[cfg(feature = "web_admin_panel")]
            {
                self.web_admin_panel_service_configuration
                    .as_ref()
                    .map(|web_admin_panel_config| format!("http://{}", web_admin_panel_config.addr))
                    .into_iter()
                    .collect()
            }
            #[cfg(not(feature = "web_admin_panel"))]
            {
                Vec::new()
            }
        };

        let cors_allowed_hosts_arc = Arc::new(
            self.configuration
                .cors_allowed_hosts
                .iter()
                .cloned()
                .chain(web_admin_panel_cors_allowed_hosts)
                .collect::<Vec<String>>(),
        );

        let app_data = Data::new(AppData {
            agent_controller_pool: self.agent_controller_pool.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.configuration.clone(),
            shutdown: shutdown.clone(),
        });

        let bind_addr = self.configuration.addr;

        let server = HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::api::post_continue_from_conversation_history::register)
                .configure(http_route::api::post_continue_from_raw_prompt::register)
                .configure(http_route::api::post_generate_embedding_batch::register)
                .configure(http_route::api::ws_inference_socket::register)
        })
        .workers(HTTP_WORKERS)
        .shutdown_signal(async move {
            shutdown.cancelled().await;
        })
        .shutdown_timeout(self.shutdown_options.cooperative_deadline.as_secs())
        .disable_signals()
        .bind(bind_addr)
        .with_context(|| format!("Unable to bind balancer inference service to {bind_addr}"))?;

        server.run().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::Context as _;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;
    use trzcina::ServiceShutdownOptions;

    use super::InferenceService;
    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use crate::resolved_socket_addr::ResolvedSocketAddr;
    #[cfg(feature = "web_admin_panel")]
    use crate::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;
    #[cfg(feature = "web_admin_panel")]
    use crate::web_admin_panel_service::template_data::TemplateData;

    fn build_service(addr: SocketAddr) -> InferenceService {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        InferenceService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
                Duration::from_secs(30),
                32,
            )),
            configuration: InferenceServiceConfiguration {
                addr,
                cors_allowed_hosts: vec!["http://127.0.0.1:8080".to_owned()],
                inference_item_timeout: Duration::from_secs(30),
            },
            shutdown_options: ServiceShutdownOptions::default(),
            #[cfg(feature = "web_admin_panel")]
            web_admin_panel_service_configuration: Some(WebAdminPanelServiceConfiguration {
                addr: SocketAddr::from(([127, 0, 0, 1], 8081)),
                template_data: TemplateData {
                    buffered_request_timeout: Duration::from_secs(30),
                    compat_openai_addr: None,
                    inference_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                    },
                    management_addr: ResolvedSocketAddr {
                        input_addr: "127.0.0.1:0".to_owned(),
                        socket_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                    },
                    max_buffered_requests: 32,
                    statsd_addr: None,
                    statsd_prefix: "paddler".to_owned(),
                    statsd_reporting_interval: Duration::from_secs(10),
                },
            }),
        }
    }

    #[actix_web::test]
    async fn run_returns_error_when_address_is_already_in_use() -> anyhow::Result<()> {
        let occupied_listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
        let occupied_addr = occupied_listener.local_addr()?;

        let service = Box::new(build_service(occupied_addr));

        let error = service
            .run(CancellationToken::new())
            .await
            .err()
            .context("binding to an already-occupied address must fail")?;
        let io_error = error
            .downcast_ref::<std::io::Error>()
            .context("the bind failure must surface as an I/O error")?;

        assert_eq!(io_error.kind(), std::io::ErrorKind::AddrInUse);

        Ok(())
    }
}
