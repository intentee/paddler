use anyhow::Result;
use paddler::agent_desired_model::AgentDesiredModel;
use paddler::balancer_desired_state::BalancerDesiredState;
use paddler::inference_parameters::InferenceParameters;

use crate::cluster_handle::ClusterHandle;
use crate::current_test_device::current_test_device;
use crate::model_card::ModelCard;
use crate::model_card::qwen3_embedding_0_6b::qwen3_embedding_0_6b;
use crate::qwen3_embedding_cluster_params::Qwen3EmbeddingClusterParams;
use crate::start_subprocess_cluster::start_subprocess_cluster;
use crate::subprocess_cluster_params::SubprocessClusterParams;

pub async fn start_subprocess_cluster_with_qwen3_embedding(
    Qwen3EmbeddingClusterParams {
        agents,
        buffered_request_timeout,
        inference_parameters,
        max_buffered_requests,
    }: Qwen3EmbeddingClusterParams,
) -> Result<ClusterHandle> {
    let ModelCard {
        gpu_layer_count,
        reference,
    } = qwen3_embedding_0_6b();

    let test_device = current_test_device()?;
    test_device.require_available()?;
    let device_offload_parameters =
        test_device.inference_parameters_for_full_offload(gpu_layer_count);

    let inference_parameters_with_offload = InferenceParameters {
        n_gpu_layers: device_offload_parameters.n_gpu_layers,
        ..inference_parameters
    };

    start_subprocess_cluster(SubprocessClusterParams {
        agents,
        buffered_request_timeout,
        desired_state: Some(BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: inference_parameters_with_offload,
            model: AgentDesiredModel::HuggingFace(reference),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }),
        max_buffered_requests,
        wait_for_slots_ready: true,
        ..SubprocessClusterParams::default()
    })
    .await
}
