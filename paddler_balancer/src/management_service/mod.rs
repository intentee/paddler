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
use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
use crate::create_cors_middleware::create_cors_middleware;
use crate::embedding_sender_collection::EmbeddingSenderCollection;
use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
use crate::http_route as common_http_route;
use crate::management_service::app_data::AppData;
use crate::management_service::configuration::Configuration as ManagementServiceConfiguration;
use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
use crate::serve_http_until_shutdown::serve_http_until_shutdown;
use crate::state_database::StateDatabase;
#[cfg(feature = "web_admin_panel")]
use crate::web_admin_panel_service::configuration::Configuration as WebAdminPanelServiceConfiguration;

#[cfg(feature = "web_admin_panel")]
fn collect_web_admin_panel_cors_allowed_hosts(
    web_admin_panel_service_configuration: Option<&WebAdminPanelServiceConfiguration>,
) -> Vec<String> {
    web_admin_panel_service_configuration
        .map(|web_admin_panel_config| format!("http://{}", web_admin_panel_config.addr))
        .into_iter()
        .collect()
}

const HTTP_WORKERS: usize = 2;

pub struct ManagementService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub chat_template_override_sender_collection: Arc<ChatTemplateOverrideSenderCollection>,
    pub configuration: ManagementServiceConfiguration,
    pub embedding_sender_collection: Arc<EmbeddingSenderCollection>,
    pub generate_tokens_sender_collection: Arc<GenerateTokensSenderCollection>,
    pub graceful_http_shutdown: bool,
    pub model_metadata_sender_collection: Arc<ModelMetadataSenderCollection>,
    pub shutdown_options: ServiceShutdownOptions,
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

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let web_admin_panel_cors_allowed_hosts: Vec<String> = {
            #[cfg(feature = "web_admin_panel")]
            {
                collect_web_admin_panel_cors_allowed_hosts(
                    self.web_admin_panel_service_configuration.as_ref(),
                )
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

        let bind_addr = self.configuration.addr;

        let server = HttpServer::new(move || {
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
        })
        .workers(HTTP_WORKERS)
        .shutdown_timeout(self.shutdown_options.cooperative_deadline.as_secs())
        .disable_signals()
        .bind(bind_addr)
        .with_context(|| format!("Unable to bind balancer management service to {bind_addr}"))?
        .run();

        serve_http_until_shutdown(server, shutdown, self.graceful_http_shutdown).await?;

        Ok(())
    }
}

#[cfg(all(test, feature = "web_admin_panel"))]
mod tests {
    use std::net::SocketAddr;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::Result;
    use tokio::sync::broadcast;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;
    use trzcina::ServiceShutdownOptions;

    use crate::agent_controller_pool::AgentControllerPool;
    use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
    use crate::buffered_request_manager::BufferedRequestManager;
    use crate::chat_template_override_sender_collection::ChatTemplateOverrideSenderCollection;
    use crate::embedding_sender_collection::EmbeddingSenderCollection;
    use crate::generate_tokens_sender_collection::GenerateTokensSenderCollection;
    use crate::management_service::configuration::Configuration as ManagementServiceConfiguration;
    use crate::model_metadata_sender_collection::ModelMetadataSenderCollection;
    use crate::resolved_socket_addr::ResolvedSocketAddr;
    use crate::state_database::memory::Memory;
    use crate::web_admin_panel_service::template_data::TemplateData;
    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::ManagementService;
    use super::WebAdminPanelServiceConfiguration;
    use super::collect_web_admin_panel_cors_allowed_hosts;

    fn build_service(addr: SocketAddr) -> ManagementService {
        let agent_controller_pool = Arc::new(AgentControllerPool::default());
        let (balancer_desired_state_notify_tx, _balancer_desired_state_notify_rx) =
            broadcast::channel(1);

        ManagementService {
            agent_controller_pool: agent_controller_pool.clone(),
            balancer_applicable_state_holder: Arc::new(BalancerApplicableStateHolder::default()),
            buffered_request_manager: Arc::new(BufferedRequestManager::new(
                agent_controller_pool,
                Duration::from_secs(30),
                32,
            )),
            chat_template_override_sender_collection: Arc::new(
                ChatTemplateOverrideSenderCollection::default(),
            ),
            configuration: ManagementServiceConfiguration {
                addr,
                cors_allowed_hosts: vec!["http://127.0.0.1:8080".to_owned()],
            },
            embedding_sender_collection: Arc::new(EmbeddingSenderCollection::default()),
            generate_tokens_sender_collection: Arc::new(GenerateTokensSenderCollection::default()),
            model_metadata_sender_collection: Arc::new(ModelMetadataSenderCollection::default()),
            graceful_http_shutdown: true,
            shutdown_options: ServiceShutdownOptions::default(),
            state_database: Arc::new(Memory::new(
                balancer_desired_state_notify_tx,
                BalancerDesiredState::default(),
            )),
            statsd_prefix: "paddler".to_owned(),
            web_admin_panel_service_configuration: None,
        }
    }

    fn make_resolved_socket_addr(input_addr: &str) -> Result<ResolvedSocketAddr> {
        Ok(ResolvedSocketAddr {
            input_addr: input_addr.to_owned(),
            socket_addr: input_addr.parse()?,
        })
    }

    fn make_web_admin_panel_configuration(
        addr: SocketAddr,
    ) -> Result<WebAdminPanelServiceConfiguration> {
        Ok(WebAdminPanelServiceConfiguration {
            addr,
            template_data: TemplateData {
                buffered_request_timeout: Duration::from_secs(1),
                compat_openai_addr: None,
                inference_addr: make_resolved_socket_addr("127.0.0.1:8081")?,
                management_addr: make_resolved_socket_addr("127.0.0.1:8082")?,
                max_buffered_requests: 1,
                statsd_addr: None,
                statsd_prefix: "paddler".to_owned(),
                statsd_reporting_interval: Duration::from_secs(1),
            },
        })
    }

    #[test]
    fn builds_http_origin_from_web_admin_panel_addr() -> Result<()> {
        let configuration = make_web_admin_panel_configuration("127.0.0.1:9000".parse()?)?;

        let allowed_hosts = collect_web_admin_panel_cors_allowed_hosts(Some(&configuration));

        assert_eq!(allowed_hosts, vec!["http://127.0.0.1:9000".to_owned()]);

        Ok(())
    }

    #[test]
    fn yields_no_hosts_when_web_admin_panel_is_absent() {
        let allowed_hosts = collect_web_admin_panel_cors_allowed_hosts(None);

        assert!(allowed_hosts.is_empty());
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
