pub mod app_data;
pub mod configuration;
pub mod http_route;
pub mod inference_mode_routes;

use std::net::SocketAddr;
use std::sync::Arc;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_messaging::inference_mode::InferenceMode;

use crate::agent_controller_pool::AgentControllerPool;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cors_allowed_hosts_with_web_admin_panel::cors_allowed_hosts_with_web_admin_panel;
use crate::create_cors_middleware::create_cors_middleware;
use crate::http_listener::HttpListener;
use crate::http_route::get_health::get_health;
use crate::inference_service::app_data::AppData;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::inference_service::inference_mode_routes::inference_mode_routes;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;

pub struct InferenceService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub configuration: InferenceServiceConfiguration,
    pub http_listener: HttpListener,
    pub inference_mode: InferenceMode,
    pub web_admin_panel_addr: Option<SocketAddr>,
}

#[async_trait]
impl Service for InferenceService {
    fn name(&self) -> &'static str {
        "balancer::inference_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let inference_mode = self.inference_mode;
        let cors_allowed_hosts_arc = cors_allowed_hosts_with_web_admin_panel(
            &self.configuration.cors_allowed_hosts,
            self.web_admin_panel_addr,
        );

        let app_data = Data::new(AppData {
            agent_controller_pool: self.agent_controller_pool.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.configuration.clone(),
            shutdown: shutdown.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                        .app_data(app_data.clone())
                        .configure(get_health)
                        .configure(move |service_config| {
                            inference_mode_routes(inference_mode, service_config);
                        })
                },
                http_listener: self.http_listener,
                service_name,
                worker_count: 16,
            },
        )
        .await
    }
}
