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

use crate::http_listener::HttpListener;
use crate::serve_http_until_shutdown::serve_http_until_shutdown;

pub struct HttpServiceRunner<TAppFactory> {
    pub app_factory: TAppFactory,
    pub http_listener: HttpListener,
    pub service_name: &'static str,
    pub worker_count: usize,
}

impl<TAppFactory> HttpServiceRunner<TAppFactory> {
    /// # Errors
    ///
    /// Fails when the HTTP server cannot listen on the bound listener, and when serving stops with an error.
    pub async fn run<TAppEntry, TResponseBody>(
        self,
        cancellation_token: CancellationToken,
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
        let Self {
            app_factory,
            http_listener:
                HttpListener {
                    local_addr,
                    tcp_listener,
                },
            service_name,
            worker_count,
        } = self;
        let server = HttpServer::new(app_factory)
            .workers(worker_count)
            .keep_alive(KeepAlive::Disabled)
            .h1_allow_half_closed(false)
            .disable_signals()
            .listen(tcp_listener)
            .with_context(|| format!("Unable to listen for {service_name} on {local_addr}"))?
            .run();

        serve_http_until_shutdown(cancellation_token, server)
            .await
            .context(format!("Unable to serve {service_name} on {local_addr}"))
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::App;
    use anyhow::Result;
    use tokio::join;
    use tokio_util::sync::CancellationToken;

    use super::HttpServiceRunner;
    use crate::http_listener::HttpListener;

    #[actix_web::test]
    async fn a_bound_service_stops_cleanly_when_its_token_is_cancelled() -> Result<()> {
        let cancellation_token = CancellationToken::new();
        let requested_shutdown = cancellation_token.clone();

        let (serve_result, ()) = join!(
            HttpServiceRunner {
                app_factory: App::new,
                http_listener: HttpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                    .expect("an ephemeral loopback port must be bindable"),
                service_name: "balancer::test_service",
                worker_count: 1,
            }
            .run(cancellation_token),
            async move { requested_shutdown.cancel() },
        );

        serve_result
    }
}
