use url::Url;

use paddler_messaging::api_path::ApiPath;

use crate::error::Error;
use crate::error::Result;

pub fn url(input: Url) -> Result<Url> {
    let websocket_scheme = match input.scheme() {
        "http" | "ws" => "ws",
        "https" | "wss" => "wss",
        rejected_scheme => {
            return Err(Error::InferenceSocketUrlSchemeRejected {
                scheme: rejected_scheme.to_owned(),
                url: input.to_string(),
            });
        }
    };
    let mut url = input;

    url.set_scheme(websocket_scheme)
        .map_err(|()| Error::InferenceSocketUrlSchemeRejected {
            scheme: websocket_scheme.to_owned(),
            url: url.to_string(),
        })?;
    url.set_path(ApiPath::INFERENCE_SOCKET);

    Ok(url)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use paddler_messaging::api_path::ApiPath;

    use super::url;
    use crate::error::Error;

    #[test]
    fn http_becomes_ws() {
        let result = url(Url::parse("http://localhost:8080/some/path").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), ApiPath::INFERENCE_SOCKET);
        assert_eq!(result.host_str(), Some("localhost"));
        assert_eq!(result.port(), Some(8080));
    }

    #[test]
    fn https_becomes_wss() {
        let result = url(Url::parse("https://example.com/ignored").unwrap()).unwrap();

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), ApiPath::INFERENCE_SOCKET);
    }

    #[test]
    fn ws_scheme_preserved() {
        let result = url(Url::parse("ws://localhost:9090").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), ApiPath::INFERENCE_SOCKET);
    }

    #[test]
    fn original_path_replaced() {
        let result = url(Url::parse("http://host/deeply/nested/path?query=1").unwrap()).unwrap();

        assert_eq!(result.path(), ApiPath::INFERENCE_SOCKET);
    }

    #[test]
    fn rejects_a_scheme_that_is_not_http_or_websocket() {
        assert!(matches!(
            url(Url::parse("ftp://host/models").unwrap()),
            Err(Error::InferenceSocketUrlSchemeRejected { scheme, url })
                if scheme == "ftp" && url == "ftp://host/models"
        ));
    }
}
