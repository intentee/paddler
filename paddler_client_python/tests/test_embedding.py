from paddler_client.embedding import Embedding
from paddler_client.pooling_type import PoolingType


def test_embedding_deserialization() -> None:
    embedding = Embedding.model_validate(
        {
            "embedding": [0.1, 0.2, 0.3],
            "normalization_method": "L2",
            "pooling_type": "Mean",
            "source_document_id": "doc-1",
        }
    )

    assert embedding.embedding == [0.1, 0.2, 0.3]
    assert embedding.normalization_method.variant == "L2"
    assert embedding.pooling_type == PoolingType.MEAN
    assert embedding.source_document_id == "doc-1"


def test_embedding_with_rms_norm_deserialization() -> None:
    embedding = Embedding.model_validate(
        {
            "embedding": [1.0],
            "normalization_method": {"RmsNorm": {"epsilon": 1e-6}},
            "pooling_type": "Cls",
            "source_document_id": "doc-2",
        }
    )

    assert embedding.normalization_method.variant == "RmsNorm"
    assert embedding.normalization_method.epsilon == 1e-6
