use std::sync::Arc;

use actix_web::dev::Server;
use tokio_util::sync::CancellationToken;

use crate::awaitable_counter::AwaitableCounter;

pub async fn serve_http_until_shutdown(
    server: Server,
    shutdown: CancellationToken,
    drain_counter: Arc<AwaitableCounter>,
) -> std::io::Result<()> {
    let server_handle = server.handle();

    tokio::spawn(async move {
        shutdown.cancelled().await;
        drain_counter.wait_for_zero().await;
        server_handle.stop(false).await;
    });

    server.await
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::App;
    use actix_web::HttpServer;
    use tokio_util::sync::CancellationToken;

    use super::*;

    #[actix_web::test]
    async fn stops_after_the_token_is_cancelled_when_no_requests_are_in_flight() {
        let server = HttpServer::new(App::new)
            .workers(1)
            .disable_signals()
            .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .expect("the server must bind a loopback port")
            .run();

        let shutdown = CancellationToken::new();
        let requested_shutdown = shutdown.clone();
        let drain_counter = Arc::new(AwaitableCounter::default());

        tokio::spawn(async move {
            requested_shutdown.cancel();
        });

        serve_http_until_shutdown(server, shutdown, drain_counter)
            .await
            .expect("the server must stop without error");
    }
}
