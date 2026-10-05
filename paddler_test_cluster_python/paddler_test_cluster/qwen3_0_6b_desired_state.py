from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.huggingface_model_reference import HuggingFaceModelReference
from paddler_client.inference_parameters import InferenceParameters

from paddler_test_cluster.all_gpu_layers import ALL_GPU_LAYERS

QWEN3_0_6B_DESIRED_STATE = BalancerDesiredState(
    inference_parameters=InferenceParameters(
        min_p=0.0,
        n_gpu_layers=ALL_GPU_LAYERS,
        penalty_frequency=0.0,
        penalty_presence=0.0,
        penalty_repeat=1.0,
        temperature=0.0,
        top_k=1,
        top_p=1.0,
    ),
    model=AgentDesiredModel.from_huggingface(
        HuggingFaceModelReference(
            filename="Qwen3-0.6B-Q8_0.gguf",
            repo_id="Qwen/Qwen3-0.6B-GGUF",
            revision="main",
        )
    ),
)
