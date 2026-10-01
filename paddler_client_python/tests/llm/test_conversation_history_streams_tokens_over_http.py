from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.inference_message import InferenceMessageKind

MAX_TOKENS = 8


async def test_conversation_history_streams_tokens_over_http(
    qwen3_cluster: BalancerAddresses,
) -> None:
    async with ClientInference(url=qwen3_cluster.inference_url) as client:
        messages = [
            message
            async for message in client.post_continue_from_conversation_history(
                ContinueFromConversationHistoryParams(
                    add_generation_prompt=True,
                    conversation_history=[
                        ConversationMessage(content="Say hello.", role="user")
                    ],
                    enable_thinking=False,
                    max_tokens=MAX_TOKENS,
                )
            )
        ]

    *tokens, done = messages

    assert {token.kind for token in tokens} == {InferenceMessageKind.CONTENT_TOKEN}
    assert done.kind == InferenceMessageKind.DONE
    assert len(tokens) == MAX_TOKENS
