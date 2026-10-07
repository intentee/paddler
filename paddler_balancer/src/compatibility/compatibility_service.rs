use std::sync::Arc;

use actix_web::App;
use actix_web::middleware::from_fn;
use actix_web::web::Data;
use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use trzcina::Service;

use crate::balancer_applicable_state_holder::BalancerApplicableStateHolder;
use crate::buffered_request_manager::BufferedRequestManager;
use crate::compatibility::attach_request_id::attach_request_id;
use crate::compatibility::compatibility_app_data::CompatibilityAppData;
use crate::compatibility::create_compatibility_cors_middleware::create_compatibility_cors_middleware;
use crate::compatibility::serves_compatibility_layer::ServesCompatibilityLayer;
use crate::http_listener::HttpListener;
use crate::http_route::get_health::get_health;
use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::run_http_service::run_http_service;
use crate::run_http_service_parameters::RunHttpServiceParameters;

pub struct CompatibilityService<TCompatibilityLayer> {
    pub balancer_applicable_state_holder: Arc<BalancerApplicableStateHolder>,
    pub buffered_request_manager: Arc<BufferedRequestManager>,
    pub compatibility_layer: TCompatibilityLayer,
    pub http_listener: HttpListener,
    pub inference_service_configuration: InferenceServiceConfiguration,
}

#[async_trait]
impl<TCompatibilityLayer: ServesCompatibilityLayer> Service
    for CompatibilityService<TCompatibilityLayer>
{
    fn name(&self) -> &'static str {
        TCompatibilityLayer::SERVICE_NAME
    }

    async fn run(self: Box<Self>, shutdown: CancellationToken) -> Result<()> {
        let service_name = self.name();
        let cors_allowed_hosts_arc = Arc::new(
            self.inference_service_configuration
                .cors_allowed_hosts
                .clone(),
        );
        let app_data = Data::new(CompatibilityAppData {
            balancer_applicable_state_holder: self.balancer_applicable_state_holder.clone(),
            buffered_request_manager: self.buffered_request_manager.clone(),
            inference_service_configuration: self.inference_service_configuration.clone(),
            shutdown: shutdown.clone(),
        });

        run_http_service(
            shutdown,
            RunHttpServiceParameters {
                app_factory: move || {
                    App::new()
                        .wrap(from_fn(attach_request_id::<TCompatibilityLayer>))
                        .wrap(create_compatibility_cors_middleware::<TCompatibilityLayer>(
                            &cors_allowed_hosts_arc,
                        ))
                        .app_data(app_data.clone())
                        .configure(get_health)
                        .configure(TCompatibilityLayer::configure)
                },
                http_listener: self.http_listener,
                service_name,
                worker_count: 16,
            },
        )
        .await
    }
}
