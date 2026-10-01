pub mod app_data;
pub mod configuration;
pub mod esbuild_meta_contents;
pub mod http_route;
pub mod template_data;

use std::str::FromStr as _;
use std::sync::Arc;

use actix_web::App;
use actix_web::web::Data;
use anyhow::Context as _;
use anyhow::Result;
use async_trait::async_trait;
use esbuild_metafile::EsbuildMetaFile;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::http_listener::HttpListener;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;
use crate::web_admin_panel_service::app_data::AppData;
use crate::web_admin_panel_service::esbuild_meta_contents::ESBUILD_META_CONTENTS;
use crate::web_admin_panel_service::http_route::favicon::favicon;
use crate::web_admin_panel_service::http_route::home::home;
use crate::web_admin_panel_service::http_route::static_files::static_files;
use crate::web_admin_panel_service::template_data::TemplateData;

pub struct WebAdminPanelService {
    pub http_listener: HttpListener,
    pub template_data: TemplateData,
}

#[async_trait]
impl Service for WebAdminPanelService {
    fn name(&self) -> &'static str {
        "balancer::web_admin_panel_service"
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let esbuild_metafile = EsbuildMetaFile::from_str(ESBUILD_META_CONTENTS)
            .context("the embedded esbuild metafile does not describe the dashboard assets")?;
        let app_data: Data<AppData> = Data::new(AppData {
            esbuild_metafile: Arc::new(esbuild_metafile),
            template_data: self.template_data,
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .app_data(app_data.clone())
                        .configure(favicon)
                        .configure(static_files)
                        .configure(home)
                },
                http_listener: self.http_listener,
                service_name,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use anyhow::Result;
    use tokio::join;
    use tokio_util::sync::CancellationToken;
    use trzcina::Service as _;

    use super::WebAdminPanelService;
    use crate::http_listener::HttpListener;
    use crate::resolved_socket_addr::ResolvedSocketAddr;
    use crate::web_admin_panel_service::template_data::TemplateData;

    fn build_service() -> WebAdminPanelService {
        let ephemeral_loopback_addr = SocketAddr::from(([127, 0, 0, 1], 0));

        WebAdminPanelService {
            http_listener: HttpListener::bind(ephemeral_loopback_addr)
                .expect("an ephemeral loopback port must be bindable"),
            template_data: TemplateData {
                buffered_request_timeout: Duration::from_secs(30),
                compat_openai_addr: None,
                inference_addr: ResolvedSocketAddr::from(ephemeral_loopback_addr),
                management_addr: ResolvedSocketAddr::from(ephemeral_loopback_addr),
                max_buffered_requests: 32,
                statsd_prefix: "paddler".to_owned(),
                statsd_service_configuration: None,
            },
        }
    }

    #[actix_web::test]
    async fn run_serves_until_shutdown_is_requested() -> Result<()> {
        let service = Box::new(build_service());
        let shutdown = CancellationToken::new();
        let requested_shutdown = shutdown.clone();

        let (run_result, ()) = join!(
            service.run(shutdown),
            async move { requested_shutdown.cancel() }
        );

        run_result
    }
}
