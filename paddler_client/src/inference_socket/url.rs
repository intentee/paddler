use url::Url;

use crate::error::Error;
use crate::error::Result;

pub fn url(input: Url) -> Result<Url> {
    let mut socket_url = input;

    let websocket_scheme = match socket_url.scheme() {
        "http" | "ws" => "ws",
        "https" | "wss" => "wss",
        unsupported => {
            return Err(Error::InferenceSocketUnsupportedScheme {
                scheme: unsupported.to_owned(),
            });
        }
    };

    let _ = socket_url.set_scheme(websocket_scheme);
    socket_url.set_path("/api/v1/inference_socket");

    Ok(socket_url)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::url;
    use crate::error::Error;

    #[test]
    fn http_becomes_ws() {
        let result = url(Url::parse("http://localhost:8080/some/path").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");
        assert_eq!(result.host_str(), Some("localhost"));
        assert_eq!(result.port(), Some(8080));
    }

    #[test]
    fn https_becomes_wss() {
        let result = url(Url::parse("https://example.com/ignored").unwrap()).unwrap();

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), "/api/v1/inference_socket");
    }

    #[test]
    fn ws_scheme_preserved() {
        let result = url(Url::parse("ws://localhost:9090").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");
    }

    #[test]
    fn wss_scheme_preserved() {
        let result = url(Url::parse("wss://localhost:9090").unwrap()).unwrap();

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), "/api/v1/inference_socket");
    }

    #[test]
    fn original_path_replaced() {
        let result = url(Url::parse("http://host/deeply/nested/path?query=1").unwrap()).unwrap();

        assert_eq!(result.path(), "/api/v1/inference_socket");
    }

    #[test]
    fn rejects_an_unsupported_scheme() {
        let error = url(Url::parse("ftp://host/model").unwrap()).unwrap_err();

        assert!(matches!(
            error,
            Error::InferenceSocketUnsupportedScheme { scheme } if scheme == "ftp"
        ));
    }
}
