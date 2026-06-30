use anyhow::Result;
use paddler_client::error::Error;
use paddler_client_tests::inference_client_for::inference_client_for;
use paddler_test_fixture::http_header::HttpHeader;
use paddler_test_fixture::http_response_spec::HttpResponseSpec;
use paddler_test_fixture::local_http_fixture::LocalHttpFixture;

const ALLOWED_ORIGIN: &str = "http://example.com";

fn allow_origin_header(value: Vec<u8>) -> HttpHeader {
    HttpHeader {
        name: "Access-Control-Allow-Origin".to_owned(),
        value,
    }
}

#[tokio::test]
async fn cors_preflight_returns_the_origin_echoed_by_the_server() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_with_headers(vec![
        allow_origin_header(ALLOWED_ORIGIN.as_bytes().to_vec()),
    ]))
    .await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let preflight = client.cors_preflight(ALLOWED_ORIGIN).await?;

    assert_eq!(preflight.status, 200);
    assert_eq!(preflight.allow_origin, ALLOWED_ORIGIN);

    Ok(())
}

#[tokio::test]
async fn cors_preflight_errors_when_the_allow_origin_header_is_missing() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(Vec::new())).await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let outcome = client.cors_preflight(ALLOWED_ORIGIN).await;

    assert!(matches!(outcome, Err(Error::CorsAllowOriginMissing)));

    Ok(())
}

#[tokio::test]
async fn cors_preflight_errors_when_the_allow_origin_header_is_not_ascii() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_with_headers(vec![
        allow_origin_header(vec![0xE9]),
    ]))
    .await?;
    let client = inference_client_for(fixture.base_url().parse()?);

    let outcome = client.cors_preflight(ALLOWED_ORIGIN).await;

    assert!(matches!(
        outcome,
        Err(Error::CorsAllowOriginNotAscii { .. })
    ));

    Ok(())
}

#[tokio::test]
async fn cors_preflight_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = inference_client_for("http://127.0.0.1:1".parse()?);

    assert!(client.cors_preflight(ALLOWED_ORIGIN).await.is_err());

    Ok(())
}
