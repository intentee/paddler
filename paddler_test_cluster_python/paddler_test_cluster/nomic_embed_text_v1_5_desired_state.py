from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.huggingface_model_reference import HuggingFaceModelReference
from paddler_client.inference_parameters import InferenceParameters

NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE = BalancerDesiredState(
    inference_parameters=InferenceParameters(
        context_size=2048,
        enable_embeddings=True,
        n_gpu_layers=999,
    ),
    model=AgentDesiredModel.from_huggingface(
        HuggingFaceModelReference(
            filename="nomic-embed-text-v1.5.Q2_K.gguf",
            repo_id="nomic-ai/nomic-embed-text-v1.5-GGUF",
            revision="main",
        )
    ),
)
