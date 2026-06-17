use reqwest::Client;
use reqwest::Method;
use reqwest::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use reqwest::header::ACCESS_CONTROL_REQUEST_METHOD;
use reqwest::header::ORIGIN;
use url::Url;

use crate::cors_preflight_response::CorsPreflightResponse;
use crate::error::Error;
use crate::error::Result;
use crate::format_api_url::format_api_url;

pub async fn cors_preflight(
    http_client: &Client,
    base_url: &Url,
    origin: &str,
) -> Result<CorsPreflightResponse> {
    let response = http_client
        .request(Method::OPTIONS, format_api_url(base_url, "/health"))
        .header(ORIGIN, origin)
        .header(ACCESS_CONTROL_REQUEST_METHOD, Method::GET.as_str())
        .send()
        .await?;

    let status = response.status().as_u16();
    let allow_origin = response
        .headers()
        .get(ACCESS_CONTROL_ALLOW_ORIGIN)
        .ok_or(Error::CorsAllowOriginMissing)?
        .to_str()
        .map_err(|source| Error::CorsAllowOriginNotAscii { source })?
        .to_owned();

    Ok(CorsPreflightResponse {
        allow_origin,
        status,
    })
}
