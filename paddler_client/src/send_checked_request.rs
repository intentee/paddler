use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::StatusCode;

use crate::error::Error;
use crate::error::Result;

pub async fn send_checked_request(
    url: String,
    request_builder: RequestBuilder,
) -> Result<Response> {
    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();

            if status.is_success() {
                Ok(response)
            } else if status == StatusCode::SERVICE_UNAVAILABLE {
                Err(Error::ServiceUnavailable { url })
            } else {
                Err(Error::UnexpectedResponseStatus { status, url })
            }
        }
        Err(source) if source.is_connect() => Err(Error::Connect { url, source }),
        Err(source) => Err(Error::Http(source)),
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use reqwest::StatusCode;
    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;

    use super::send_checked_request;
    use crate::error::Error;

    async fn serve_one_response(raw_response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the fixture server must bind a loopback port");
        let address = listener
            .local_addr()
            .expect("the fixture server must report its address");

        tokio::spawn(async move {
            let (mut connection, _peer_address) = listener
                .accept()
                .await
                .expect("the fixture server must accept the connection");
            let mut request_bytes = [0_u8; 1024];
            let request_byte_count = connection
                .read(&mut request_bytes)
                .await
                .expect("the fixture server must read the request");

            assert!(
                request_byte_count > 0,
                "the fixture server must receive the request head"
            );

            connection
                .write_all(raw_response.as_bytes())
                .await
                .expect("the fixture server must write the response");
        });

        format!("http://{address}/health")
    }

    #[tokio::test]
    async fn returns_the_response_for_a_success_status() {
        let url = serve_one_response("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK").await;
        let request_builder = Client::new().get(&url);

        let response = send_checked_request(url, request_builder)
            .await
            .expect("a success status must be returned to the caller");

        assert_eq!(
            response
                .text()
                .await
                .expect("the response body must be readable"),
            "OK"
        );
    }

    #[tokio::test]
    async fn maps_service_unavailable_to_its_own_variant() {
        let url =
            serve_one_response("HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n")
                .await;
        let request_builder = Client::new().get(&url);

        assert!(matches!(
            send_checked_request(url, request_builder).await,
            Err(Error::ServiceUnavailable { .. })
        ));
    }

    #[tokio::test]
    async fn maps_any_other_failure_status_to_the_unexpected_status_variant() {
        let url = serve_one_response("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
        let request_builder = Client::new().get(&url);

        assert!(matches!(
            send_checked_request(url, request_builder).await,
            Err(Error::UnexpectedResponseStatus {
                status: StatusCode::NOT_FOUND,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn a_refused_connection_maps_to_the_connect_variant() {
        let url = "http://127.0.0.1:1/health".to_owned();
        let request_builder = Client::new().get(&url);

        assert!(matches!(
            send_checked_request(url, request_builder).await,
            Err(Error::Connect { .. })
        ));
    }
}
