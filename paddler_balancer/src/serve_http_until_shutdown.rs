use std::pin::pin;

use actix_web::dev::Server;
use tokio_util::sync::CancellationToken;

pub async fn serve_http_until_shutdown(
    shutdown: CancellationToken,
    server: Server,
) -> std::io::Result<()> {
    let server_handle = server.handle();
    let mut server = pin!(server);

    tokio::select! {
        server_result = server.as_mut() => return server_result,
        () = shutdown.cancelled() => {}
    }

    let (server_result, ()) = tokio::join!(server.as_mut(), server_handle.stop(true));

    server_result
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::App;
    use actix_web::HttpServer;
    use actix_web::dev::Server;
    use tokio_util::sync::CancellationToken;

    use super::serve_http_until_shutdown;

    fn bound_server() -> Server {
        HttpServer::new(App::new)
            .disable_signals()
            .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .expect("the server must bind a loopback port")
            .run()
    }

    #[actix_web::test]
    async fn a_cancelled_token_stops_the_server() {
        let shutdown = CancellationToken::new();
        let requested_shutdown = shutdown.clone();

        tokio::spawn(async move {
            requested_shutdown.cancel();
        });

        serve_http_until_shutdown(shutdown, bound_server())
            .await
            .expect("a cancelled token must stop the server without error");
    }

    #[actix_web::test]
    async fn a_server_that_stops_on_its_own_reports_its_own_result() {
        let server = bound_server();
        let server_handle = server.handle();

        let (serve_result, ()) = tokio::join!(
            serve_http_until_shutdown(CancellationToken::new(), server),
            server_handle.stop(false),
        );

        serve_result.expect("a server that stops on its own must report its own result");
    }
}
