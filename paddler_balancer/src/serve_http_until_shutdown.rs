use actix_web::dev::Server;
use tokio_util::sync::CancellationToken;

pub async fn serve_http_until_shutdown(
    server: Server,
    shutdown: CancellationToken,
    graceful: bool,
) -> std::io::Result<()> {
    let server_handle = server.handle();

    tokio::spawn(async move {
        shutdown.cancelled().await;
        server_handle.stop(graceful).await;
    });

    server.await
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::App;
    use actix_web::HttpServer;
    use tokio_util::sync::CancellationToken;

    use super::serve_http_until_shutdown;

    #[actix_web::test]
    async fn a_forced_shutdown_stops_the_server() {
        let server = HttpServer::new(App::new)
            .disable_signals()
            .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .expect("the server must bind a loopback port")
            .run();

        let shutdown = CancellationToken::new();
        let requested_shutdown = shutdown.clone();

        tokio::spawn(async move {
            requested_shutdown.cancel();
        });

        serve_http_until_shutdown(server, shutdown, false)
            .await
            .expect("the server must stop without error");
    }
}
