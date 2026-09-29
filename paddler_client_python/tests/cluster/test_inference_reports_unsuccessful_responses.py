from typing import TYPE_CHECKING

import pytest
from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.embedding_input_document import EmbeddingInputDocument
from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.error import HttpError
from paddler_client.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

NOT_FOUND = 404


async def test_inference_reports_unsuccessful_responses(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientInference(
        url=f"{cluster_without_agents.inference_url}/not-a-paddler-route"
    ) as client:

        async def first_generated_message() -> object:
            return await anext(
                client.post_continue_from_conversation_history(
                    ContinueFromConversationHistoryParams(
                        add_generation_prompt=True,
                        conversation_history=[
                            ConversationMessage(content="Hello", role="user")
                        ],
                        enable_thinking=False,
                        max_tokens=1,
                    )
                )
            )

        async def first_embedding_message() -> object:
            return await anext(
                client.generate_embedding_batch(
                    GenerateEmbeddingBatchParams(
                        input_batch=[
                            EmbeddingInputDocument(content="Hello", id="document")
                        ],
                        normalization_method=EmbeddingNormalizationMethod.none(),
                    )
                )
            )

        requests: list[Callable[[], Awaitable[object]]] = [
            client.get_health,
            first_generated_message,
            first_embedding_message,
        ]

        for request in requests:
            with pytest.raises(HttpError) as unsuccessful:
                await request()

            assert unsuccessful.value.status_code == NOT_FOUND
