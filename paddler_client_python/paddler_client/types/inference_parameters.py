from pydantic import BaseModel

from paddler_client.types.pooling_type import PoolingType


class InferenceParameters(BaseModel):
    batch_n_tokens: int = 512
    context_size: int = 8192
    embedding_n_seq_max: int = 16
    enable_embeddings: bool = False
    image_resize_to_fit: int = 1024
    min_p: float = 0.05
    penalty_frequency: float = 0.0
    penalty_last_n: int = -1
    penalty_presence: float = 0.8
    penalty_repeat: float = 1.1
    pooling_type: PoolingType = PoolingType.LAST
    temperature: float = 0.8
    top_k: int = 80
    top_p: float = 0.8
