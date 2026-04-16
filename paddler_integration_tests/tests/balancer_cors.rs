#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::pick_free_port::pick_free_port;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_inference_cors_headers() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let allowed_origin = "http://example.com";

    let management_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);
    let inference_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);
    let compat_openai_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);

    let _balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr,
        inference_addr: inference_addr.clone(),
        inference_cors_allowed_hosts: vec![allowed_origin.to_owned()],
        inference_item_timeout: None,
        management_addr,
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: format!(
            "file://{}",
            state_db
                .path()
                .to_str()
                .context("temp file path is not valid UTF-8")?
        ),
    })
    .await
    .context("failed to spawn balancer")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .request(
            reqwest::Method::OPTIONS,
            format!("http://{inference_addr}/health"),
        )
        .header("Origin", allowed_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .context("preflight request should succeed")?;

    assert_eq!(response.status(), 200);

    let cors_origin = response
        .headers()
        .get("access-control-allow-origin")
        .context("should have Access-Control-Allow-Origin header")?
        .to_str()
        .context("header should be valid string")?;

    assert_eq!(cors_origin, allowed_origin);

    Ok(())
}

#[tokio::test]
#[file_serial]
async fn test_management_cors_headers() -> Result<()> {
    let state_db = NamedTempFile::new().context("failed to create temp file")?;
    let allowed_origin = "http://example.com";

    let management_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);
    let inference_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);
    let compat_openai_addr = format!("127.0.0.1:{}", pick_free_port().context("pick port")?);

    let _balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr,
        inference_addr,
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: management_addr.clone(),
        management_cors_allowed_hosts: vec![allowed_origin.to_owned()],
        max_buffered_requests: 10,
        state_database_url: format!(
            "file://{}",
            state_db
                .path()
                .to_str()
                .context("temp file path is not valid UTF-8")?
        ),
    })
    .await
    .context("failed to spawn balancer")?;

    let http_client = reqwest::Client::new();

    let response = http_client
        .request(
            reqwest::Method::OPTIONS,
            format!("http://{management_addr}/health"),
        )
        .header("Origin", allowed_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .context("preflight request should succeed")?;

    assert_eq!(response.status(), 200);

    let cors_origin = response
        .headers()
        .get("access-control-allow-origin")
        .context("should have Access-Control-Allow-Origin header")?
        .to_str()
        .context("header should be valid string")?;

    assert_eq!(cors_origin, allowed_origin);

    Ok(())
}
