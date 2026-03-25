#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;

use paddler_integration_tests::BALANCER_INFERENCE_ADDR;
use paddler_integration_tests::BALANCER_MANAGEMENT_ADDR;
use paddler_integration_tests::BALANCER_OPENAI_ADDR;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer::ManagedBalancerParams;
use serial_test::file_serial;
use tempfile::NamedTempFile;

#[tokio::test]
#[file_serial]
async fn test_inference_cors_headers() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let allowed_origin = "http://example.com";

    let _balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_string(),
        inference_cors_allowed_hosts: vec![allowed_origin.to_string()],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 10,
        state_database_url: format!("file://{}", state_db.path().to_str().unwrap()),
    })
    .await
    .expect("failed to spawn balancer");

    let http_client = reqwest::Client::new();

    let response = http_client
        .request(
            reqwest::Method::OPTIONS,
            format!("http://{BALANCER_INFERENCE_ADDR}/health"),
        )
        .header("Origin", allowed_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .expect("preflight request should succeed");

    assert_eq!(response.status(), 200);

    let cors_origin = response
        .headers()
        .get("access-control-allow-origin")
        .expect("should have Access-Control-Allow-Origin header")
        .to_str()
        .expect("header should be valid string");

    assert_eq!(cors_origin, allowed_origin);
}

#[tokio::test]
#[file_serial]
async fn test_management_cors_headers() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let allowed_origin = "http://example.com";

    let _balancer = ManagedBalancer::spawn(ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: BALANCER_OPENAI_ADDR.to_owned(),
        inference_addr: BALANCER_INFERENCE_ADDR.to_string(),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: BALANCER_MANAGEMENT_ADDR.to_string(),
        management_cors_allowed_hosts: vec![allowed_origin.to_string()],
        max_buffered_requests: 10,
        state_database_url: format!("file://{}", state_db.path().to_str().unwrap()),
    })
    .await
    .expect("failed to spawn balancer");

    let http_client = reqwest::Client::new();

    let response = http_client
        .request(
            reqwest::Method::OPTIONS,
            format!("http://{BALANCER_MANAGEMENT_ADDR}/health"),
        )
        .header("Origin", allowed_origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .expect("preflight request should succeed");

    assert_eq!(response.status(), 200);

    let cors_origin = response
        .headers()
        .get("access-control-allow-origin")
        .expect("should have Access-Control-Allow-Origin header")
        .to_str()
        .expect("header should be valid string");

    assert_eq!(cors_origin, allowed_origin);
}
