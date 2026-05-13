#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use paddler_tests::agent_config::AgentConfig;
use paddler_tests::collect_embedding_results::collect_embedding_results;
use paddler_tests::inference_http_client::InferenceHttpClient;
use paddler_tests::start_in_process_embedding_cluster::start_in_process_embedding_cluster;
use paddler_types::embedding_input_document::EmbeddingInputDocument;
use paddler_types::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_types::inference_parameters::InferenceParameters;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use reqwest::Client;

const N_BATCH: u32 = 64;

#[serial_test::file_serial(model_load, path => "../target/model_load.lock")]
#[tokio::test(flavor = "multi_thread")]
async fn agent_embedding_document_exceeds_n_batch() -> Result<()> {
    let cluster = start_in_process_embedding_cluster(
        InferenceParameters {
            n_batch: N_BATCH as usize,
            context_size: 4096,
            enable_embeddings: true,
            ..InferenceParameters::default()
        },
        AgentConfig::single(1),
    )
    .await?;

    let inference_client =
        InferenceHttpClient::new(Client::new(), cluster.addresses.inference_base_url()?);

    let huge_content = "The quick brown fox jumps over the lazy dog. ".repeat(40);

    let stream = inference_client
        .post_generate_embedding_batch(&GenerateEmbeddingBatchParams {
            input_batch: vec![
                EmbeddingInputDocument {
                    content: "ok".to_owned(),
                    id: "tiny".to_owned(),
                },
                EmbeddingInputDocument {
                    content: huge_content,
                    id: "huge".to_owned(),
                },
            ],
            normalization_method: EmbeddingNormalizationMethod::None,
        })
        .await?;

    let collected = collect_embedding_results(stream).await?;

    assert!(
        collected.saw_done,
        "stream must terminate with Done even when one document is oversized",
    );
    assert!(
        collected.errors.is_empty(),
        "no generic EmbeddingResult::Error events should be emitted; got {:?}",
        collected.errors,
    );

    assert_eq!(
        collected.oversized_documents.len(),
        1,
        "exactly one DocumentExceedsBatchSize event expected; got {:?}",
        collected
            .oversized_documents
            .iter()
            .map(|details| &details.source_document_id)
            .collect::<Vec<_>>(),
    );

    let oversized = &collected.oversized_documents[0];

    assert_eq!(oversized.source_document_id, "huge");
    assert_eq!(oversized.n_batch, N_BATCH);
    assert!(
        oversized.document_tokens > oversized.n_batch,
        "document_tokens ({}) must exceed n_batch ({}) for the assertion to be meaningful",
        oversized.document_tokens,
        oversized.n_batch,
    );

    assert_eq!(
        collected.embeddings.len(),
        1,
        "the small document must still be embedded; got {:?}",
        collected
            .embeddings
            .iter()
            .map(|produced| &produced.embedding.source_document_id)
            .collect::<Vec<_>>(),
    );
    assert_eq!(collected.embeddings[0].embedding.source_document_id, "tiny",);

    cluster.shutdown().await?;

    Ok(())
}
