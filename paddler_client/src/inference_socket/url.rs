use url::Url;

use crate::error::Error;
use crate::error::Result;

pub fn url(input: Url) -> Result<Url> {
    let mut url = input;

    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => other,
    };

    let scheme = scheme.to_owned();

    url.set_scheme(&scheme)
        .map_err(|()| Error::Other(format!("Failed to set URL scheme to '{scheme}'")))?;
    url.set_path("/api/v1/inference_socket");

    Ok(url)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::url;

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
    fn original_path_replaced() {
        let result = url(Url::parse("http://host/deeply/nested/path?query=1").unwrap()).unwrap();

        assert_eq!(result.path(), "/api/v1/inference_socket");
    }
}
