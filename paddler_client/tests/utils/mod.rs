use paddler_client::PaddlerClient;
use url::Url;

pub fn create_paddler_client() -> PaddlerClient {
    let management_url = Url::parse("http://127.0.0.1:8060").expect("valid management URL");
    let inference_url = Url::parse("http://127.0.0.1:8061").expect("valid inference URL");

    PaddlerClient::new(inference_url, management_url, 1)
}

pub async fn get_first_agent_id() -> String {
    let client = create_paddler_client();
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
