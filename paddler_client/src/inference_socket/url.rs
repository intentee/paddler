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
    use crate::error::Result;

    #[test]
    fn test_http_becomes_ws() -> Result<()> {
        let input = Url::parse("http://localhost:8080/some/path")?;
        let result = url(input)?;

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");
        assert_eq!(result.host_str(), Some("localhost"));
        assert_eq!(result.port(), Some(8080));

        Ok(())
    }

    #[test]
    fn test_https_becomes_wss() -> Result<()> {
        let input = Url::parse("https://example.com/ignored")?;
        let result = url(input)?;

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }

    #[test]
    fn test_ws_scheme_preserved() -> Result<()> {
        let input = Url::parse("ws://localhost:9090")?;
        let result = url(input)?;

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }

    #[test]
    fn test_original_path_replaced() -> Result<()> {
        let input = Url::parse("http://host/deeply/nested/path?query=1")?;
        let result = url(input)?;

        assert_eq!(result.path(), "/api/v1/inference_socket");

        Ok(())
    }
}
