use url::Url;

use crate::error::Error;
use crate::error::Result;

pub fn inference_socket_url(url: Url) -> Result<Url> {
    let mut url = url;

    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => other,
    };

    let scheme = scheme.to_string();

    url.set_scheme(&scheme)
        .map_err(|()| Error::Other(format!("Failed to set URL scheme to '{scheme}'")))?;
    url.set_path("/api/v1/inference_socket");

    Ok(url)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::inference_socket_url;
    use crate::error::Result;

    #[test]
    fn test_http_becomes_ws() -> Result<()> {
        let url = Url::parse("http://localhost:8080/some/path")?;
        let result = inference_socket_url(url)?;

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");
        assert_eq!(result.host_str(), Some("localhost"));
        assert_eq!(result.port(), Some(8080));

        Ok(())
    }

    #[test]
    fn test_https_becomes_wss() -> Result<()> {
        let url = Url::parse("https://example.com/ignored")?;
        let result = inference_socket_url(url)?;

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }

    #[test]
    fn test_ws_scheme_preserved() -> Result<()> {
        let url = Url::parse("ws://localhost:9090")?;
        let result = inference_socket_url(url)?;

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }

    #[test]
    fn test_original_path_replaced() -> Result<()> {
        let url = Url::parse("http://host/deeply/nested/path?query=1")?;
        let result = inference_socket_url(url)?;

        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }
}
