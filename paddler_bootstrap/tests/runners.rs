use std::fs;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler::balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;
use paddler::balancer::state_database::File as StateDatabaseFile;
use paddler::balancer::state_database::StateDatabase;
use paddler::balancer::state_database_type::StateDatabaseType;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::chat_template::ChatTemplate;
use paddler::inference_parameters::InferenceParameters;
use paddler::request_params::ContinueFromRawPromptParams;
use paddler_bootstrap::ServiceShutdownOptions;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_bootstrap::balancer_runner::BalancerRunnerParams;
use tempfile::NamedTempFile;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

fn pick_free_loopback_addr() -> Result<SocketAddr> {
    let probe =
        TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral loopback port")?;
    let addr = probe
        .local_addr()
        .context("failed to read ephemeral local_addr")?;

    drop(probe);

    Ok(addr)
}

async fn wait_until_bound(addr: SocketAddr) -> Result<()> {
    loop {
        if TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

fn make_balancer_runner_params(
    management_addr: SocketAddr,
    inference_addr: SocketAddr,
    cancellation_token: CancellationToken,
) -> BalancerRunnerParams {
    BalancerRunnerParams {
        buffered_request_timeout: Duration::from_secs(10),
        inference_service_configuration: InferenceServiceConfiguration {
            addr: inference_addr,
            cors_allowed_hosts: vec![],
            inference_item_timeout: Duration::from_secs(30),
        },
        management_service_configuration: ManagementServiceConfiguration {
            addr: management_addr,
            cors_allowed_hosts: vec![],
        },
        max_buffered_requests: 30,
        openai_service_configuration: None,
        cancellation_token,
        shutdown_options: ServiceShutdownOptions::default(),
        state_database_type: StateDatabaseType::Memory(Box::default()),
        statsd_prefix: "paddler_bootstrap_test_".to_owned(),
        statsd_service_configuration: None,
        #[cfg(feature = "web_admin_panel")]
        web_admin_panel_service_configuration: None,
    }
}

fn make_agent_runner_params(
    management_addr: SocketAddr,
    cancellation_token: CancellationToken,
) -> AgentRunnerParams {
    AgentRunnerParams {
        agent_name: Some("test-agent".to_owned()),
        management_address: management_addr.to_string(),
        cancellation_token,
        slots: 1,
    }
}

#[tokio::test]
async fn balancer_runner_exits_when_dropped() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let runner = BalancerRunner::start(make_balancer_runner_params(
        management_addr,
        inference_addr,
        CancellationToken::new(),
    ))
    .await?;

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after balancer runner drop")?;
    TcpListener::bind(inference_addr)
        .context("inference port is still held after balancer runner drop")?;

    Ok(())
}

#[tokio::test]
async fn balancer_runner_exits_on_explicit_cancel() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let runner = BalancerRunner::start(make_balancer_runner_params(
        management_addr,
        inference_addr,
        CancellationToken::new(),
    ))
    .await?;

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    runner.cancel();
    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after explicit cancel")?;

    Ok(())
}

#[tokio::test]
async fn balancer_runner_cancels_from_parent_token() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let parent = CancellationToken::new();

    let runner = BalancerRunner::start(make_balancer_runner_params(
        management_addr,
        inference_addr,
        parent.clone(),
    ))
    .await?;

    wait_until_bound(management_addr).await?;
    wait_until_bound(inference_addr).await?;

    parent.cancel();
    drop(runner);

    TcpListener::bind(management_addr)
        .context("management port is still held after parent cancel")?;

    Ok(())
}

#[tokio::test]
async fn balancer_runner_preserves_persisted_desired_state() -> Result<()> {
    let state_db_file = NamedTempFile::new()?;
    let state_db_path = state_db_file.path().to_path_buf();

    let persisted_state = BalancerDesiredState {
        chat_template_override: Some(ChatTemplate {
            content: "persisted-chat-template".to_owned(),
        }),
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("persisted-model".to_owned()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    {
        let (tx, _rx) = broadcast::channel(100);
        let seeded_database = StateDatabaseFile::new(tx, state_db_path.clone());

        seeded_database
            .store_balancer_desired_state(&persisted_state)
            .await?;
    }

    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let mut params =
        make_balancer_runner_params(management_addr, inference_addr, CancellationToken::new());

    params.state_database_type = StateDatabaseType::File(state_db_path.clone());

    let runner = BalancerRunner::start(params).await?;

    wait_until_bound(management_addr).await?;

    assert_eq!(runner.initial_desired_state, persisted_state);

    runner.cancel();
    drop(runner);

    let (tx, _rx) = broadcast::channel(100);
    let verify_database = StateDatabaseFile::new(tx, state_db_path);
    let on_disk = verify_database.read_balancer_desired_state().await?;

    assert_eq!(on_disk, persisted_state);

    Ok(())
}

#[tokio::test]
async fn agent_runner_exits_when_dropped() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;

    let runner = AgentRunner::start(make_agent_runner_params(
        management_addr,
        CancellationToken::new(),
    ));

    drop(runner);

    Ok(())
}

#[tokio::test]
async fn agent_runner_cancels_from_parent_token() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;

    let parent = CancellationToken::new();

    let runner = AgentRunner::start(make_agent_runner_params(management_addr, parent.clone()));

    parent.cancel();
    drop(runner);

    Ok(())
}

#[tokio::test]
async fn in_flight_request_is_released_when_balancer_shuts_down() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let mut params =
        make_balancer_runner_params(management_addr, inference_addr, CancellationToken::new());

    // The request never finds an agent, so it stays buffered (in flight) until this long
    // timeout — which is far longer than the short shutdown deadline below.
    params.buffered_request_timeout = Duration::from_mins(1);

    // A short, non-zero deadline: if the in-flight request fails to observe shutdown, actix
    // runs its graceful drain to this deadline and trzcina aborts the service — the bug.
    params.shutdown_options = ServiceShutdownOptions {
        cooperative_deadline: Duration::from_secs(2),
        abort_deadline: Duration::from_secs(2),
    };

    let mut runner = BalancerRunner::start(params).await?;

    wait_until_bound(inference_addr).await?;

    // No agents are registered, so this request buffers and holds its streaming response open.
    let held_response = reqwest::Client::new()
        .post(format!(
            "http://{inference_addr}/api/v1/continue_from_raw_prompt"
        ))
        .json(&ContinueFromRawPromptParams {
            grammar: None,
            max_tokens: 10,
            raw_prompt: "hold the connection open during shutdown".to_owned(),
        })
        .send()
        .await
        .context("inference request headers should be received")?;

    runner.cancel();

    let body = held_response
        .text()
        .await
        .context("held in-flight response body should be readable")?;

    let shutdown_result = runner.wait_for_completion().await;

    assert!(
        body.contains("shutting down"),
        "in-flight request must be released with a shutdown error, got body: {body:?}"
    );
    assert!(
        shutdown_result.is_ok(),
        "balancer shutdown must complete cleanly below the deadline, got: {shutdown_result:?}"
    );

    Ok(())
}

#[tokio::test]
async fn agent_runner_completes_after_explicit_cancel() -> Result<()> {
    let management_addr = pick_free_loopback_addr()?;

    let mut runner = AgentRunner::start(make_agent_runner_params(
        management_addr,
        CancellationToken::new(),
    ));

    runner.cancel();
    runner.wait_for_completion().await?;

    Ok(())
}

#[tokio::test]
async fn balancer_runner_fails_to_start_when_state_database_file_is_corrupt() -> Result<()> {
    let corrupt_state_database = NamedTempFile::new()?;
    fs::write(
        corrupt_state_database.path(),
        b"this is not a valid state database",
    )?;

    let management_addr = pick_free_loopback_addr()?;
    let inference_addr = pick_free_loopback_addr()?;

    let mut params =
        make_balancer_runner_params(management_addr, inference_addr, CancellationToken::new());

    params.state_database_type =
        StateDatabaseType::File(corrupt_state_database.path().to_path_buf());

    let start_result = BalancerRunner::start(params).await;

    assert!(start_result.is_err());

    Ok(())
}
