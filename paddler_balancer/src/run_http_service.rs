use std::fmt::Debug;

use actix_http::Request;
use actix_http::Response;
use actix_http::body::MessageBody;
use actix_service::IntoServiceFactory;
use actix_service::Service;
use actix_service::ServiceFactory;
use actix_web::Error;
use actix_web::HttpServer;
use actix_web::dev::AppConfig;
use actix_web::http::KeepAlive;
use anyhow::Context as _;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::run_http_service_parameters::RunHttpServiceParameters;
use crate::serve_http_until_shutdown::serve_http_until_shutdown;

pub async fn run_http_service<TAppFactory, TServiceFactoryInput, TServiceFactory, TResponseBody>(
    cancellation_token: CancellationToken,
    RunHttpServiceParameters {
        app_factory,
        bind_addr,
        service_name,
        worker_count,
    }: RunHttpServiceParameters<TAppFactory>,
) -> Result<()>
where
    TAppFactory: Fn() -> TServiceFactoryInput + Send + Clone + 'static,
    TServiceFactoryInput: IntoServiceFactory<TServiceFactory, Request>,
    TServiceFactory: ServiceFactory<Request, Config = AppConfig> + 'static,
    TServiceFactory::Error: Into<Error> + 'static,
    TServiceFactory::InitError: Debug,
    TServiceFactory::Response: Into<Response<TResponseBody>> + 'static,
    TServiceFactory::Service: 'static,
    <TServiceFactory::Service as Service<Request>>::Future: 'static,
    TResponseBody: MessageBody + 'static,
{
    let server = HttpServer::new(app_factory)
        .workers(worker_count)
        .keep_alive(KeepAlive::Disabled)
        .disable_signals()
        .bind(bind_addr)
        .with_context(|| format!("Unable to bind {service_name} to {bind_addr}"))?
        .run();

    serve_http_until_shutdown(cancellation_token, server)
        .await
        .context(format!("Unable to serve {service_name} on {bind_addr}"))
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::App;
    use anyhow::Result;
    use tokio_util::sync::CancellationToken;

    use super::run_http_service;
    use crate::run_http_service_parameters::RunHttpServiceParameters;

    #[actix_web::test]
    async fn a_bound_service_serves_until_the_token_is_cancelled() -> Result<()> {
        let cancellation_token = CancellationToken::new();
        let requested_shutdown = cancellation_token.clone();

        let (serve_result, ()) = tokio::join!(
            run_http_service(
                cancellation_token,
                RunHttpServiceParameters {
                    app_factory: App::new,
                    bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                    service_name: "balancer::test_service",
                    worker_count: 1,
                },
            ),
            async move { requested_shutdown.cancel() },
        );

        serve_result
    }
}
