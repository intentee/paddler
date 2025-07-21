use std::net::SocketAddr;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpServer;
use async_trait::async_trait;
#[cfg(unix)]
use pingora::server::ListenFds;
use pingora::server::ShutdownWatch;
use pingora::services::Service;

use crate::balancer::http_route;
use crate::balancer::upstream_peer_pool::UpstreamPeerPool;

fn create_cors_middleware(allowed_hosts: Arc<Vec<String>>) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "OPTIONS"])
        .allowed_headers(vec![
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
        ])
        .max_age(3600);

    for host in allowed_hosts.iter() {
        cors = cors.allowed_origin(host);
    }

    cors
}

pub struct ManagementService {
    addr: SocketAddr,
    management_cors_allowed_hosts: Arc<Vec<String>>,
    #[cfg(feature = "web_dashboard")]
    management_dashboard_enable: bool,
    metrics_endpoint_enable: bool,
    upstream_peers: Arc<UpstreamPeerPool>,
}

impl ManagementService {
    pub fn new(
        addr: SocketAddr,
        management_cors_allowed_hosts: Vec<String>,
        #[cfg(feature = "web_dashboard")] management_dashboard_enable: bool,
        metrics_endpoint_enable: bool,
        upstream_peers: Arc<UpstreamPeerPool>,
    ) -> Self {
        ManagementService {
            addr,
            management_cors_allowed_hosts: Arc::new(management_cors_allowed_hosts),
            #[cfg(feature = "web_dashboard")]
            management_dashboard_enable,
            metrics_endpoint_enable,
            upstream_peers,
        }
    }
}

#[async_trait]
impl Service for ManagementService {
    async fn start_service(
        &mut self,
        #[cfg(unix)] _fds: Option<ListenFds>,
        mut _shutdown: ShutdownWatch,
        _listeners_per_fd: usize,
    ) {
        #[cfg(feature = "web_dashboard")]
        let management_dashboard_enable = self.management_dashboard_enable;

        let metrics_endpoint_enable = self.metrics_endpoint_enable;

        let upstream_peers: Data<UpstreamPeerPool> = self.upstream_peers.clone().into();
        let management_cors_allowed_hosts = self.management_cors_allowed_hosts.clone();

        HttpServer::new(move || {
            #[allow(unused_mut)]
            let mut app = App::new()
                .wrap(create_cors_middleware(
                    management_cors_allowed_hosts.clone(),
                ))
                .app_data(upstream_peers.clone())
                .configure(http_route::api::get_agents::register)
                .configure(http_route::api::get_agents_stream::register)
                .configure(http_route::api::post_agent_status_update::register);

            #[cfg(feature = "web_dashboard")]
            if management_dashboard_enable {
                app = app
                    .configure(http_route::dashboard::register)
                    .configure(http_route::static_files::register);
            }

            if metrics_endpoint_enable {
                app = app.configure(http_route::api::get_metrics::register)
            }

            app
        })
        .bind(self.addr)
        .expect("Unable to bind server to address")
        .run()
        .await
        .expect("Server unexpectedly stopped");
    }

    fn name(&self) -> &str {
        "management"
    }

    fn threads(&self) -> Option<usize> {
        Some(1)
    }
}
