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
