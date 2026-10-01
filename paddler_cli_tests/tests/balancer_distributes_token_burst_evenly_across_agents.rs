#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use futures_util::StreamExt as _;
use futures_util::future;
use tokio_util::sync::CancellationToken;

use paddler_cli_tests::start_subprocess_cluster_with_qwen3::start_subprocess_cluster_with_qwen3;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::unending_generation::unending_generation;

const AGENT_COUNT: usize = 4;
const SLOTS_PER_AGENT: u16 = 1;

#[tokio::test(flavor = "multi_thread")]
async fn balancer_distributes_token_burst_evenly_across_agents() {
    let cluster = start_subprocess_cluster_with_qwen3(
        env!("CARGO_BIN_EXE_paddler_cluster_node"),
        AgentConfig::uniform(AGENT_COUNT, SLOTS_PER_AGENT),
    )
    .await
    .expect("the cluster must start");
    let generation_cancellation = CancellationToken::new();
    let mut streams = future::try_join_all((0..AGENT_COUNT).map(|_generation_index| {
        cluster.continue_from_raw_prompt_stream(
            generation_cancellation.clone(),
            &unending_generation(),
        )
    }))
    .await
    .expect("every concurrent request must succeed");
    let mut producers = BTreeSet::new();

    for stream in &mut streams {
        match stream
            .next()
            .await
            .expect("every held generation must stream a first message")
            .expect("the message must be readable")
        {
            InferenceMessage::Response(ResponseEnvelope {
                generated_by: Some(producer),
                ..
            }) => {
                producers.insert(producer);
            }
            unexpected_message => {
                panic!("a held generation began with {unexpected_message:?}");
            }
        }
    }

    generation_cancellation.cancel();
    drop(streams);

    assert_eq!(producers.len(), AGENT_COUNT);

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
