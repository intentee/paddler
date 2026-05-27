#![cfg(all(
    unix,
    feature = "tests_that_use_compiled_paddler",
    feature = "tests_that_use_llms"
))]

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use anyhow::Context as _;
use anyhow::Result;
use nix::sys::signal::Signal;
use paddler_cli_tests::agent_config::AgentConfig;
use paddler_cli_tests::collect_embedding_results::collect_embedding_results;
use paddler_cli_tests::inference_http_client::InferenceHttpClient;
use paddler_cli_tests::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use paddler_cli_tests::start_subprocess_cluster_with_qwen3_embedding::start_subprocess_cluster_with_qwen3_embedding;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio_util::sync::CancellationToken;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn paddler_subprocess_cluster_does_not_leak_sigurg_to_parent_process() -> Result<()> {
    let observed_sigurg_count = Arc::new(AtomicUsize::new(0));
    let observer_shutdown = CancellationToken::new();

    let observer_count = observed_sigurg_count.clone();
    let observer_token = observer_shutdown.clone();
    let mut sigurg_stream = signal(SignalKind::from_raw(Signal::SIGURG as i32))
        .context("failed to install SIGURG observer on the test process")?;

    let observer_handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                () = observer_token.cancelled() => break,
                signal_event = sigurg_stream.recv() => match signal_event {
                    Some(()) => {
                        observer_count.fetch_add(1, Ordering::SeqCst);
                    }
                    None => break,
                },
            }
        }
    });

    let cluster = start_subprocess_cluster_with_qwen3_embedding(Qwen3EmbeddingClusterParams {
        agents: AgentConfig::uniform(2, 2),
        inference_parameters: InferenceParameters {
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        ..Qwen3EmbeddingClusterParams::default()
    })
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let input_batch: Vec<EmbeddingInputDocument> = (0..4)
        .map(|document_index| EmbeddingInputDocument {
            content: format!("SIGURG regression document number {document_index:02}"),
            id: format!("doc-{document_index}"),
        })
        .collect();
    let params = GenerateEmbeddingBatchParams {
        input_batch,
        normalization_method: EmbeddingNormalizationMethod::None,
    };

    let stream = inference_client
        .post_generate_embedding_batch(&params)
        .await?;
    let collected = collect_embedding_results(stream).await?;

    assert_eq!(collected.embeddings.len(), 4);
    assert!(collected.errors.is_empty());

    cluster.shutdown().await?;

    observer_shutdown.cancel();
    observer_handle
        .await
        .context("SIGURG observer task panicked")?;

    let final_sigurg_count = observed_sigurg_count.load(Ordering::SeqCst);

    assert_eq!(
        final_sigurg_count, 0,
        "paddler subprocesses leaked {final_sigurg_count} SIGURG signals to the parent process; \
         this would kill bash test harness loops that rely on SIGURG's default ignore action being honored. \
         The observer ran throughout cluster startup, an embedding inference, and cluster shutdown."
    );

    Ok(())
}
