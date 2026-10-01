pub mod app_data;
pub mod configuration;
pub mod http_route;

use std::net::SocketAddr;
use std::sync::Arc;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use paddler_state_database::state_database::StateDatabase;

use crate::agent_controller_pool::AgentControllerPool;
use crate::agent_response_senders::AgentResponseSenders;
use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::cors_allowed_hosts_with_web_admin_panel::cors_allowed_hosts_with_web_admin_panel;
use crate::create_cors_middleware::create_cors_middleware;
use crate::http_listener::HttpListener;
use crate::http_route::get_health::get_health;
use crate::management_service::app_data::AppData;
use crate::management_service::configuration::Configuration as ManagementServiceConfiguration;
use crate::management_service::http_route::api::get_agents::get_agents;
use crate::management_service::http_route::api::get_agents_stream::get_agents_stream;
use crate::management_service::http_route::api::get_balancer_applicable_state::get_balancer_applicable_state;
use crate::management_service::http_route::api::get_balancer_desired_state::get_balancer_desired_state;
use crate::management_service::http_route::api::get_buffered_requests::get_buffered_requests;
use crate::management_service::http_route::api::get_buffered_requests_stream::get_buffered_requests_stream;
use crate::management_service::http_route::api::get_chat_template_override::get_chat_template_override;
use crate::management_service::http_route::api::get_model_metadata::get_model_metadata;
use crate::management_service::http_route::api::put_balancer_desired_state::put_balancer_desired_state;
use crate::management_service::http_route::api::ws_agent_socket::ws_agent_socket;
use crate::management_service::http_route::get_metrics::get_metrics;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;

pub struct ManagementService {
    pub agent_controller_pool: Arc<AgentControllerPool>,
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub agent_response_senders: AgentResponseSenders,
    pub configuration: ManagementServiceConfiguration,
    pub http_listener: HttpListener,
    pub state_database: Arc<dyn StateDatabase>,
    pub statsd_prefix: String,
    pub web_admin_panel_addr: Option<SocketAddr>,
}

#[async_trait]
impl Service for ManagementService {
    fn name(&self) -> &'static str {
        "balancer::management_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let cors_allowed_hosts_arc = cors_allowed_hosts_with_web_admin_panel(
            &self.configuration.cors_allowed_hosts,
            self.web_admin_panel_addr,
        );

        let app_data = Data::new(AppData {
            agent_controller_pool: self.agent_controller_pool.clone(),
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            agent_response_senders: self.agent_response_senders.clone(),
            shutdown: shutdown.clone(),
            state_database: self.state_database.clone(),
            statsd_prefix: self.statsd_prefix.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .wrap(create_cors_middleware(&cors_allowed_hosts_arc))
                        .app_data(app_data.clone())
                        .configure(get_health)
                        .configure(get_agents)
                        .configure(get_agents_stream)
                        .configure(get_balancer_applicable_state)
                        .configure(get_balancer_desired_state)
                        .configure(get_buffered_requests)
                        .configure(get_buffered_requests_stream)
                        .configure(get_chat_template_override)
                        .configure(get_model_metadata)
                        .configure(put_balancer_desired_state)
                        .configure(ws_agent_socket)
                        .configure(get_metrics)
                },
                http_listener: self.http_listener,
                service_name,
            },
        )
        .await
    }
}
