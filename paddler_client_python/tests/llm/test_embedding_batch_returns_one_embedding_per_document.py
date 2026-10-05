from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.embedding_input_document import EmbeddingInputDocument
from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)
from paddler_client.inference_message import InferenceMessageKind

DOCUMENT_IDS = ["first", "second", "third"]


async def test_embedding_batch_returns_one_embedding_per_document(
    nomic_embed_cluster: BalancerAddresses,
) -> None:
    async with ClientInference(url=nomic_embed_cluster.inference_url) as client:
        messages = [
            message
            async for message in client.generate_embedding_batch(
                GenerateEmbeddingBatchParams(
                    input_batch=[
                        EmbeddingInputDocument(
                            content=f"Document {document_id}", id=document_id
                        )
                        for document_id in DOCUMENT_IDS
                    ],
                    normalization_method=EmbeddingNormalizationMethod.l2(),
                )
            )
        ]

    embedded_document_ids = sorted(
        message.embedding_data.source_document_id
        for message in messages
        if message.kind == InferenceMessageKind.EMBEDDING and message.embedding_data
    )

    assert embedded_document_ids == sorted(DOCUMENT_IDS)
    assert messages[-1].kind == InferenceMessageKind.EMBEDDING_DONE
