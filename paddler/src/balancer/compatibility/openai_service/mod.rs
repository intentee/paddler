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

use crate::balancer::BALANCER_HTTP_SERVICE_WORKER_COUNT;
use crate::balancer::buffered_request_manager::BufferedRequestManager;
use crate::balancer::compatibility::openai_service::app_data::AppData;
use crate::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
use crate::balancer::http_route as common_http_route;
use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::create_cors_middleware::create_cors_middleware;

pub struct OpenAIService {
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub inference_service_configuration: InferenceServiceConfiguration,
    pub openai_service_configuration: OpenAIServiceConfiguration,
    pub shutdown_options: ServiceShutdownOptions,
}

#[async_trait]
impl Service for OpenAIService {
    fn name(&self) -> &'static str {
        "balancer::compatibility::openai_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let cors_allowed_hosts = self
            .inference_service_configuration
            .cors_allowed_hosts
            .clone();
        let cors_allowed_hosts_arc = Arc::new(cors_allowed_hosts);

        let app_data = Data::new(AppData {
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            shutdown: shutdown.clone(),
        });

        let bind_addr = self.openai_service_configuration.addr;

        let server = HttpServer::new(move || {
            App::new()
                .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                .app_data(app_data.clone())
                .configure(common_http_route::get_health::register)
                .configure(http_route::post_chat_completions::register)
        })
        .shutdown_signal(async move {
            shutdown.cancelled().await;
        })
        .shutdown_timeout(self.shutdown_options.cooperative_deadline.as_secs())
        .workers(BALANCER_HTTP_SERVICE_WORKER_COUNT)
        .disable_signals()
        .bind(bind_addr)
        .with_context(|| format!("Unable to bind balancer OpenAI-compat service to {bind_addr}"))?;

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

    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;
    use trzcina::ServiceShutdownOptions;

    use super::OpenAIService;
    use crate::balancer::agent_controller_pool::AgentControllerPool;
    use crate::balancer::buffered_request_manager::BufferedRequestManager;
    use crate::balancer::compatibility::openai_service::configuration::Configuration as OpenAIServiceConfiguration;
    use crate::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;

    fn build_service(addr: SocketAddr) -> OpenAIService {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());

        OpenAIService {
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
                Duration::from_secs(30),
                32,
            )),
            inference_service_configuration: InferenceServiceConfiguration {
                addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                cors_allowed_hosts: vec!["http://127.0.0.1:8080".to_owned()],
                inference_item_timeout: Duration::from_secs(30),
            },
            openai_service_configuration: OpenAIServiceConfiguration { addr },
            shutdown_options: ServiceShutdownOptions::default(),
        }
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
