use anyhow::Context as _;
use anyhow::Result;
use futures_util::StreamExt as _;
use paddler_client_tests::management_client_for::management_client_for;
use paddler_messaging::agent_controller_pool_snapshot::AgentControllerPoolSnapshot;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::buffered_request_manager_snapshot::BufferedRequestManagerSnapshot;
use paddler_test_fixture::http_header::HttpHeader;
use paddler_test_fixture::http_response_spec::HttpResponseSpec;
use paddler_test_fixture::local_http_fixture::LocalHttpFixture;

const fn empty_pool() -> AgentControllerPoolSnapshot {
    AgentControllerPoolSnapshot { agents: Vec::new() }
}

const fn buffered_requests_snapshot() -> BufferedRequestManagerSnapshot {
    BufferedRequestManagerSnapshot {
        buffered_requests_current: 0,
    }
}

fn json_body(value: &impl serde::Serialize) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

fn sse_event(payload: &[u8]) -> Vec<u8> {
    let mut body = b"data: ".to_vec();

    body.extend_from_slice(payload);
    body.extend_from_slice(b"\n\n");

    body
}

#[tokio::test]
async fn health_returns_the_server_body() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(b"OK".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert_eq!(client.health().await?, "OK");

    Ok(())
}

#[tokio::test]
async fn health_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.health().await.is_err());

    Ok(())
}

#[tokio::test]
async fn metrics_returns_the_server_body() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(b"paddler_metric 1".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert_eq!(client.metrics().await?, "paddler_metric 1");

    Ok(())
}

#[tokio::test]
async fn agents_parses_the_pool_snapshot() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(json_body(&empty_pool())?)).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.agents().await?.agents.is_empty());

    Ok(())
}

#[tokio::test]
async fn agents_errors_on_a_malformed_body() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(b"not json".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.agents().await.is_err());

    Ok(())
}

#[tokio::test]
async fn desired_state_parses_the_balancer_state() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(
        json_body(&BalancerDesiredState::default())?,
    ))
    .await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert_eq!(client.desired_state().await?, BalancerDesiredState::default());

    Ok(())
}

#[tokio::test]
async fn applicable_state_maps_null_to_none() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(b"null".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.applicable_state().await?.is_none());

    Ok(())
}

#[tokio::test]
async fn set_desired_state_succeeds_on_a_success_status() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(Vec::new())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    client.set_desired_state(&BalancerDesiredState::default()).await?;

    Ok(())
}

#[tokio::test]
async fn set_desired_state_errors_on_a_server_error_status() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::status(500, "Internal Server Error")).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.set_desired_state(&BalancerDesiredState::default()).await.is_err());

    Ok(())
}

#[tokio::test]
async fn buffered_requests_parses_the_snapshot() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(
        json_body(&buffered_requests_snapshot())?,
    ))
    .await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert_eq!(client.buffered_requests().await?.buffered_requests_current, 0);

    Ok(())
}

#[tokio::test]
async fn chat_template_override_maps_null_to_none() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(b"null".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.chat_template_override("agent-1").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn model_metadata_maps_null_to_none() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(b"null".to_vec())).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.model_metadata("agent-1").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn agents_stream_yields_parsed_snapshots() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(sse_event(&json_body(&empty_pool())?)))
            .await?;
    let client = management_client_for(fixture.base_url().parse()?);

    let mut stream = client.agents_stream().await?;
    let snapshot = stream.next().await.context("a streamed snapshot")??;

    assert!(snapshot.agents.is_empty());

    Ok(())
}

#[tokio::test]
async fn agents_stream_surfaces_a_malformed_event() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(sse_event(b"not json"))).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    let mut stream = client.agents_stream().await?;
    let outcome = stream.next().await.context("a streamed item")?;

    assert!(outcome.is_err());

    Ok(())
}

#[tokio::test]
async fn buffered_requests_stream_yields_parsed_snapshots() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_body(sse_event(
        &json_body(&buffered_requests_snapshot())?,
    )))
    .await?;
    let client = management_client_for(fixture.base_url().parse()?);

    let mut stream = client.buffered_requests_stream().await?;
    let snapshot = stream.next().await.context("a streamed snapshot")??;

    assert_eq!(snapshot.buffered_requests_current, 0);

    Ok(())
}

const UNREACHABLE_URL: &str = "http://127.0.0.1:1";

#[tokio::test]
async fn health_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = management_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.health().await.is_err());

    Ok(())
}

#[tokio::test]
async fn set_desired_state_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = management_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.set_desired_state(&BalancerDesiredState::default()).await.is_err());

    Ok(())
}

#[tokio::test]
async fn cors_preflight_returns_the_origin_echoed_by_the_server() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::ok_with_headers(vec![HttpHeader {
        name: "Access-Control-Allow-Origin".to_owned(),
        value: b"http://example.com".to_vec(),
    }]))
    .await?;
    let client = management_client_for(fixture.base_url().parse()?);

    let preflight = client.cors_preflight("http://example.com").await?;

    assert_eq!(preflight.status, 200);
    assert_eq!(preflight.allow_origin, "http://example.com");

    Ok(())
}

#[tokio::test]
async fn agents_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = management_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.agents().await.is_err());

    Ok(())
}

#[tokio::test]
async fn metrics_errors_when_the_body_is_truncated() -> Result<()> {
    let fixture = LocalHttpFixture::start(HttpResponseSpec::truncated_body()).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    assert!(client.metrics().await.is_err());

    Ok(())
}

#[tokio::test]
async fn agents_stream_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = management_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.agents_stream().await.is_err());

    Ok(())
}

#[tokio::test]
async fn buffered_requests_stream_errors_when_the_server_is_unreachable() -> Result<()> {
    let client = management_client_for(UNREACHABLE_URL.parse()?);

    assert!(client.buffered_requests_stream().await.is_err());

    Ok(())
}

#[tokio::test]
async fn buffered_requests_stream_surfaces_a_malformed_event() -> Result<()> {
    let fixture =
        LocalHttpFixture::start(HttpResponseSpec::ok_body(sse_event(b"not json"))).await?;
    let client = management_client_for(fixture.base_url().parse()?);

    let mut stream = client.buffered_requests_stream().await?;
    let outcome = stream.next().await.context("a streamed item")?;

    assert!(outcome.is_err());

    Ok(())
}
