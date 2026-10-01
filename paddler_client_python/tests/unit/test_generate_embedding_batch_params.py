from paddler_client.embedding_input_document import EmbeddingInputDocument
from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)


def test_serializes_the_embedding_batch_request() -> None:
    params = GenerateEmbeddingBatchParams(
        input_batch=[
            EmbeddingInputDocument(content="hello world", id="doc-1"),
        ],
        normalization_method=EmbeddingNormalizationMethod.l2(),
    )
    dumped = params.model_dump(mode="json")

    assert len(dumped["input_batch"]) == 1
    assert dumped["input_batch"][0]["id"] == "doc-1"
    assert dumped["normalization_method"] == "L2"
