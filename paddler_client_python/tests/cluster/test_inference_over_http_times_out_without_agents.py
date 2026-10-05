from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.inference_message import InferenceMessageKind

GATEWAY_TIMEOUT = 504


async def test_inference_over_http_times_out_without_agents(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientInference(url=cluster_without_agents.inference_url) as client:
        messages = [
            message
            async for message in client.post_continue_from_conversation_history(
                ContinueFromConversationHistoryParams(
                    add_generation_prompt=True,
                    conversation_history=[
                        ConversationMessage(content="Hello", role="user")
                    ],
                    enable_thinking=False,
                    max_tokens=1,
                )
            )
        ]

    [message] = messages

    assert message.kind == InferenceMessageKind.SERVER_ERROR
    assert message.error_code == GATEWAY_TIMEOUT
