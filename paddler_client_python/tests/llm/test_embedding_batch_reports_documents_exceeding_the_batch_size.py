from paddler_test_cluster.nomic_embed_text_v1_5_desired_state import (
    NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE,
)
from paddler_test_cluster.paddler_cluster import paddler_cluster

from paddler_client.client_inference import ClientInference
from paddler_client.embedding_input_document import EmbeddingInputDocument
from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)
from paddler_client.inference_message import InferenceMessageKind
from tests.llm.conftest import NOMIC_EMBED_AGENT

N_BATCH = 64


async def test_embedding_batch_reports_documents_exceeding_the_batch_size() -> None:
    desired_state = NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE.model_copy(
        update={
            "inference_parameters": (
                NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE.inference_parameters.model_copy(
                    update={"n_batch": N_BATCH}
                )
            )
        }
    )

    async with (
        paddler_cluster(desired_state, [NOMIC_EMBED_AGENT]) as addresses,
        ClientInference(url=addresses.inference_url) as client,
    ):
        messages = [
            message
            async for message in client.generate_embedding_batch(
                GenerateEmbeddingBatchParams(
                    input_batch=[
                        EmbeddingInputDocument(
                            content="The quick brown fox jumps over the lazy dog. "
                            * 40,
                            id="oversized",
                        )
                    ],
                    normalization_method=EmbeddingNormalizationMethod.none(),
                )
            )
        ]

    oversized_documents = [
        message.oversized_embedding_document_details
        for message in messages
        if message.kind == InferenceMessageKind.EMBEDDING_DOCUMENT_EXCEEDS_BATCH_SIZE
    ]

    assert [
        (details.source_document_id, details.n_batch)
        for details in oversized_documents
        if details is not None
    ] == [("oversized", N_BATCH)]
    assert messages[-1].kind == InferenceMessageKind.EMBEDDING_DONE
