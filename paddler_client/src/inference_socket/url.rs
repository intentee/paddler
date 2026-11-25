use url::Url;

use paddler_messaging::constants::API_PATH_INFERENCE_SOCKET;

use crate::error::Error;
use crate::result::Result;

fn websocket_scheme(scheme: &str) -> Option<&'static str> {
    match scheme {
        "http" | "ws" => Some("ws"),
        "https" | "wss" => Some("wss"),
        _ => None,
    }
}

/// # Errors
///
/// Returns [`Error::InferenceSocketUrlSchemeRejected`] when the scheme of the URL is not http, https, ws or wss.
pub fn url(input: Url) -> Result<Url> {
    let mut url = input;

    match websocket_scheme(url.scheme())
        .ok_or(())
        .and_then(|scheme| url.set_scheme(scheme))
    {
        Ok(()) => {
            url.set_path(API_PATH_INFERENCE_SOCKET);

            Ok(url)
        }
        Err(()) => Err(Error::InferenceSocketUrlSchemeRejected {
            scheme: url.scheme().to_owned(),
            url: url.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use paddler_messaging::constants::API_PATH_INFERENCE_SOCKET;

    use super::url;
    use crate::error::Error;

    #[test]
    fn an_http_url_becomes_a_ws_url_of_the_same_host_and_port() {
        let result = url(Url::parse("http://localhost:8080/some/path").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), API_PATH_INFERENCE_SOCKET);
        assert_eq!(result.host_str(), Some("localhost"));
        assert_eq!(result.port(), Some(8080));
    }

    #[test]
    fn an_https_url_becomes_a_wss_url() {
        let result = url(Url::parse("https://example.com/ignored").unwrap()).unwrap();

        assert_eq!(result.scheme(), "wss");
        assert_eq!(result.path(), API_PATH_INFERENCE_SOCKET);
    }

    #[test]
    fn a_ws_url_keeps_its_scheme_and_points_at_the_inference_socket() {
        let result = url(Url::parse("ws://localhost:9090").unwrap()).unwrap();

        assert_eq!(result.scheme(), "ws");
        assert_eq!(result.path(), API_PATH_INFERENCE_SOCKET);
    }

    #[test]
    fn a_url_path_is_replaced_by_the_inference_socket_path() {
        let result = url(Url::parse("http://host/deeply/nested/path?query=1").unwrap()).unwrap();

        assert_eq!(result.path(), API_PATH_INFERENCE_SOCKET);
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
