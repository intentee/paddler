#![cfg(feature = "integration_test_management")]

mod utils;

use futures_util::StreamExt;

async fn get_first_agent_id() -> String {
    let client = utils::create_paddler_client();
    let management = client.management();
    let snapshot = management
        .get_agents()
        .await
        .expect("get_agents must succeed");

    assert!(
        !snapshot.agents.is_empty(),
        "at least one agent must be connected"
    );

    snapshot.agents[0].id.clone()
}

#[tokio::test]
async fn test_get_agents_returns_agents() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let snapshot = management.get_agents().await?;

    assert!(
        !snapshot.agents.is_empty(),
        "expected at least one agent in the pool"
    );

    Ok(())
}

#[tokio::test]
async fn test_agents_stream_receives_snapshot() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let mut stream = management.agents_stream().await?;
    let first_event = stream
        .next()
        .await
        .expect("stream must produce at least one event")?;

    assert!(
        !first_event.agents.is_empty(),
        "first stream event must contain agents"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_balancer_desired_state() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let _state = management.get_balancer_desired_state().await?;

    Ok(())
}

#[tokio::test]
async fn test_put_balancer_desired_state_roundtrip() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let original_state = management.get_balancer_desired_state().await?;

    management
        .put_balancer_desired_state(&original_state)
        .await?;

    let restored_state = management.get_balancer_desired_state().await?;

    assert_eq!(
        original_state.use_chat_template_override,
        restored_state.use_chat_template_override,
    );

    Ok(())
}

#[tokio::test]
async fn test_get_buffered_requests() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let snapshot = management.get_buffered_requests().await?;

    assert!(
        snapshot.buffered_requests_current >= 0,
        "buffered request count must be non-negative"
    );

    Ok(())
}

#[tokio::test]
async fn test_buffered_requests_stream_receives_snapshot() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let mut stream = management.buffered_requests_stream().await?;
    let first_event = stream
        .next()
        .await
        .expect("stream must produce at least one event")?;

    assert!(
        first_event.buffered_requests_current >= 0,
        "buffered request count must be non-negative"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_metrics_returns_prometheus_format() -> paddler_client::Result<()> {
    let client = utils::create_paddler_client();
    let management = client.management();
    let metrics = management.get_metrics().await?;

    assert!(
        metrics.contains("slots_processing"),
        "metrics must contain slots_processing gauge"
    );
    assert!(
        metrics.contains("slots_total"),
        "metrics must contain slots_total gauge"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_chat_template_override() -> paddler_client::Result<()> {
    let agent_id = get_first_agent_id().await;
    let client = utils::create_paddler_client();
    let management = client.management();
    let _result = management.get_chat_template_override(&agent_id).await?;

    Ok(())
}

#[tokio::test]
async fn test_get_model_metadata() -> paddler_client::Result<()> {
    let agent_id = get_first_agent_id().await;
    let client = utils::create_paddler_client();
    let management = client.management();
    let _result = management.get_model_metadata(&agent_id).await?;

    Ok(())
}
