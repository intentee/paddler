#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;
use std::mem::discriminant;

use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;

use paddler_agent_pipeline_harness::desired_state_reconciled_discriminant::desired_state_reconciled_discriminant;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_agent_tests::swappable_model_file::SwappableModelFile;
use paddler_balancer_desired_state_fixture::desired_state_with_chat_template_override::desired_state_with_chat_template_override;
use paddler_balancer_desired_state_fixture::first_message_content_chat_template::first_message_content_chat_template;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::agent_state_application_status::AgentStateApplicationStatus;
use paddler_messaging::agent_status::AgentStatus;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::desired_state_version::DesiredStateVersion;
use paddler_messaging::model_download_status::ModelDownloadStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

#[tokio::test(flavor = "multi_thread")]
async fn a_reconfiguration_replaced_before_it_starts_yields_to_its_replacement() {
    let swappable_model_file = SwappableModelFile::link_to(
        &qwen3_5_0_8b()
            .cached_path()
            .expect("the Qwen3.5 model must be in the Hugging Face cache"),
    )
    .expect("the model must be linked into a swappable model file");
    let served_desired_state = BalancerDesiredState {
        model: swappable_model_file
            .agent_desired_model()
            .expect("the swappable model file must be at an absolute path"),
        ..qwen3_5_0_8b().into_desired_state()
    };
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(served_desired_state.clone())
        .await
        .expect("the desired state must be served");

    swappable_model_file
        .replace_with_an_unloadable_file()
        .expect("the unloadable file must replace the model");

    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(BalancerDesiredState {
                    embeddings: EmbeddingParameters {
                        pooling_type: PoolingType::Mean,
                        ..served_desired_state.embeddings.clone()
                    },
                    ..served_desired_state.clone()
                })
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    assert_eq!(
        discriminant(
            &serving_pipeline_arbiter
                .apply(desired_state_with_chat_template_override(
                    served_desired_state,
                    first_message_content_chat_template(),
                ))
                .await
        ),
        desired_state_reconciled_discriminant()
    );
    serving_pipeline_arbiter.conclude_next_startup().await;
    serving_pipeline_arbiter.conclude_next_startup().await;
    let AgentStatus {
        desired_slots_total,
        desired_state_version,
        download_status,
        issues,
        model_path,
        runtime,
        state_application_status,
        uses_chat_template_override,
    } = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(desired_slots_total, 1);
    assert_eq!(desired_state_version, DesiredStateVersion::Nothing);
    assert_eq!(download_status, ModelDownloadStatus::NotDownloading);
    assert_eq!(issues, BTreeSet::new());
    assert_eq!(
        model_path,
        Some(swappable_model_file.model_path().display().to_string())
    );
    assert!(matches!(
        runtime,
        AgentRuntimeStatus::Serving {
            capabilities: ModelCapabilities {
                decisions: DecisionsCapability::Unsupported(_),
                ..
            },
            slots_total: 1,
        }
    ));
    assert_eq!(
        state_application_status,
        AgentStateApplicationStatus::Applied
    );
    assert!(uses_chat_template_override);

    serving_pipeline_arbiter.shut_down().await;
}
