from pydantic import BaseModel, ConfigDict, Field, model_validator

from paddler_client.error import (
    BatchSizeExceedsContextSizeError,
    PenaltiesWithoutWindowError,
    PenaltyWindowWithoutPenaltiesError,
)
from paddler_client.kv_cache_dtype import KvCacheDtype
from paddler_client.pooling_type import PoolingType

I32_MAX = 2_147_483_647
NEUTRAL_PENALTY_REPEAT = 1.0


class InferenceParameters(BaseModel):
    model_config = ConfigDict(allow_inf_nan=False)

    n_batch: int = Field(default=2048, ge=1, le=I32_MAX)
    context_size: int = Field(default=8192, ge=1)
    embedding_batch_size: int = Field(default=256, ge=1)
    enable_embeddings: bool = False
    image_resize_to_fit: int = Field(default=1024, ge=1)
    k_cache_dtype: KvCacheDtype = KvCacheDtype.Q8_0
    v_cache_dtype: KvCacheDtype = KvCacheDtype.Q8_0
    min_p: float = Field(default=0.05, ge=0.0, le=1.0)
    n_gpu_layers: int = Field(default=0, ge=-1)
    penalty_frequency: float = 0.0
    penalty_last_n: int = Field(default=0, ge=0)
    penalty_presence: float = 0.0
    penalty_repeat: float = Field(default=NEUTRAL_PENALTY_REPEAT, gt=0.0)
    pooling_type: PoolingType = PoolingType.LAST
    temperature: float = Field(default=0.8, ge=0.0)
    top_k: int = Field(default=80, ge=0)
    top_p: float = Field(default=0.8, ge=0.0, le=1.0)

    @model_validator(mode="after")
    def reject_ambiguous_combinations(self) -> "InferenceParameters":
        penalties_are_neutral = (
            self.penalty_repeat == NEUTRAL_PENALTY_REPEAT
            and self.penalty_frequency == 0.0
            and self.penalty_presence == 0.0
        )

        if self.n_batch > self.context_size:
            raise BatchSizeExceedsContextSizeError(self.n_batch, self.context_size)

        if self.penalty_last_n == 0 and not penalties_are_neutral:
            raise PenaltiesWithoutWindowError

        if self.penalty_last_n > 0 and penalties_are_neutral:
            raise PenaltyWindowWithoutPenaltiesError(self.penalty_last_n)

        return self
