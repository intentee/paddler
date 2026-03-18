from pydantic import BaseModel

from paddler_client.types.embedding_input_document import EmbeddingInputDocument
from paddler_client.types.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)


class GenerateEmbeddingBatchParams(BaseModel):
    input_batch: list[EmbeddingInputDocument]
    normalization_method: EmbeddingNormalizationMethod
