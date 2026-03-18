from pydantic import BaseModel, ConfigDict

from paddler_client.types.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.types.pooling_type import PoolingType


class Embedding(BaseModel):
    model_config = ConfigDict(frozen=True)

    embedding: list[float]
    normalization_method: EmbeddingNormalizationMethod
    pooling_type: PoolingType
    source_document_id: str
