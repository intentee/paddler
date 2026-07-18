use actix_web::App;
use actix_web::Error;
use actix_web::HttpServer;
use actix_web::body::MessageBody;
use actix_web::dev::ServiceFactory;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::http::KeepAlive;
use anyhow::Context as _;
use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::run_http_service_parameters::RunHttpServiceParameters;
use crate::serve_http_until_shutdown::serve_http_until_shutdown;

pub async fn run_http_service<TAppFactory, TAppEntry, TResponseBody>(
    cancellation_token: CancellationToken,
    RunHttpServiceParameters {
        app_factory,
        bind_addr,
        service_name,
        worker_count,
    }: RunHttpServiceParameters<TAppFactory>,
) -> Result<()>
where
    TAppFactory: Fn() -> App<TAppEntry> + Send + Clone + 'static,
    TAppEntry: ServiceFactory<
            ServiceRequest,
            Config = (),
            Response = ServiceResponse<TResponseBody>,
            Error = Error,
            InitError = (),
        > + 'static,
    TResponseBody: MessageBody + 'static,
{
    let server = HttpServer::new(app_factory)
        .workers(worker_count)
        .keep_alive(KeepAlive::Disabled)
        .h1_allow_half_closed(false)
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
