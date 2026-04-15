#![cfg(feature = "tests_that_use_compiled_paddler")]

use std::time::Duration;

use paddler_integration_tests::AGENT_DESIRED_MODEL;
use paddler_integration_tests::managed_balancer::ManagedBalancer;
use paddler_integration_tests::managed_balancer_params::ManagedBalancerParams;
use paddler_integration_tests::pick_free_port::pick_free_port;
use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::chat_template::ChatTemplate;
use paddler_types::inference_parameters::InferenceParameters;
use serial_test::file_serial;
use tempfile::NamedTempFile;

fn make_balancer_params(state_db_url: String) -> ManagedBalancerParams {
    ManagedBalancerParams {
        buffered_request_timeout: Duration::from_secs(10),
        compat_openai_addr: format!("127.0.0.1:{}", pick_free_port().expect("pick port")),
        inference_addr: format!("127.0.0.1:{}", pick_free_port().expect("pick port")),
        inference_cors_allowed_hosts: vec![],
        inference_item_timeout: None,
        management_addr: format!("127.0.0.1:{}", pick_free_port().expect("pick port")),
        management_cors_allowed_hosts: vec![],
        max_buffered_requests: 30,
        state_database_url: state_db_url,
    }
}

#[tokio::test]
#[file_serial]
async fn test_desired_state_persists_across_restarts() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let mut balancer = ManagedBalancer::spawn(make_balancer_params(state_db_url.clone()))
        .await
        .expect("failed to spawn first balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    balancer
        .shutdown()
        .expect("failed to shutdown first balancer");

    let restarted_balancer = ManagedBalancer::spawn(make_balancer_params(state_db_url))
        .await
        .expect("failed to spawn second balancer");

    restarted_balancer
        .wait_for_desired_state(&desired_state)
        .await;
}

#[tokio::test]
#[file_serial]
async fn test_balancer_can_switch_model() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let balancer = ManagedBalancer::spawn(make_balancer_params(state_db_url))
        .await
        .expect("failed to spawn balancer");

    let first_desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&first_desired_state)
        .await
        .expect("failed to set first desired state");

    balancer.wait_for_desired_state(&first_desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state");

    assert_eq!(retrieved_state.model, AGENT_DESIRED_MODEL.clone());

    let second_desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AgentDesiredModel::LocalToAgent("alternative-model".to_string()),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&second_desired_state)
        .await
        .expect("failed to set second desired state");

    balancer.wait_for_desired_state(&second_desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state after switch");

    assert_eq!(
        retrieved_state.model,
        AgentDesiredModel::LocalToAgent("alternative-model".to_string())
    );
}

#[tokio::test]
#[file_serial]
async fn test_chat_template_override_persists_across_restarts() {
    let state_db = NamedTempFile::new().expect("failed to create temp file");
    let state_db_url = format!("file://{}", state_db.path().to_str().unwrap());

    let chat_template = ChatTemplate {
        content: "{% for message in messages %}{{ message.content }}{% endfor %}".to_string(),
    };

    let desired_state = BalancerDesiredState {
        chat_template_override: Some(chat_template.clone()),
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: true,
    };

    let mut balancer = ManagedBalancer::spawn(make_balancer_params(state_db_url.clone()))
        .await
        .expect("failed to spawn first balancer");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set desired state with chat template");

    balancer.wait_for_desired_state(&desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state");

    assert_eq!(
        retrieved_state.chat_template_override,
        Some(chat_template.clone())
    );
    assert!(retrieved_state.use_chat_template_override);

    balancer.shutdown().expect("failed to shutdown balancer");

    let restarted_balancer = ManagedBalancer::spawn(make_balancer_params(state_db_url))
        .await
        .expect("failed to spawn restarted balancer");

    let persisted_state = restarted_balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get persisted state after restart");

    assert_eq!(persisted_state.chat_template_override, Some(chat_template));
    assert!(persisted_state.use_chat_template_override);
}

#[tokio::test]
#[file_serial]
async fn test_desired_state_works_with_memory_storage() {
    let desired_state = BalancerDesiredState {
        chat_template_override: None,
        inference_parameters: InferenceParameters::default(),
        model: AGENT_DESIRED_MODEL.clone(),
        multimodal_projection: AgentDesiredModel::None,
        use_chat_template_override: false,
    };

    let balancer = ManagedBalancer::spawn(make_balancer_params("memory://".to_owned()))
        .await
        .expect("failed to spawn balancer with memory storage");

    balancer
        .client()
        .management()
        .put_balancer_desired_state(&desired_state)
        .await
        .expect("failed to set balancer desired state");

    balancer.wait_for_desired_state(&desired_state).await;

    let retrieved_state = balancer
        .client()
        .management()
        .get_balancer_desired_state()
        .await
        .expect("failed to get balancer desired state");

    assert_eq!(retrieved_state.model, AGENT_DESIRED_MODEL.clone());
    assert_eq!(
        retrieved_state.multimodal_projection,
        AgentDesiredModel::None
    );
    assert!(!retrieved_state.use_chat_template_override);
}
