from pydantic import BaseModel

from paddler_client.kv_cache_dtype import KvCacheDtype
from paddler_client.pooling_type import PoolingType


class InferenceParameters(BaseModel):
    n_batch: int = 2048
    context_size: int = 8192
    embedding_batch_size: int = 256
    enable_embeddings: bool = False
    image_resize_to_fit: int = 1024
    k_cache_dtype: KvCacheDtype = KvCacheDtype.Q8_0
    v_cache_dtype: KvCacheDtype = KvCacheDtype.Q8_0
    min_p: float = 0.05
    n_gpu_layers: int = 0
    penalty_frequency: float = 0.0
    penalty_last_n: int = -1
    penalty_presence: float = 0.8
    penalty_repeat: float = 1.1
    pooling_type: PoolingType = PoolingType.LAST
    temperature: float = 0.8
    top_k: int = 80
    top_p: float = 0.8
