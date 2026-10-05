import pytest
from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.error import ConnectionDroppedError

BALANCER_MAX_WEBSOCKET_FRAME_BYTES = 50 * 1024 * 1024
GATEWAY_TIMEOUT = 504


async def test_inference_socket_reconnects_after_the_balancer_closes_it(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientInference(
        url=cluster_without_agents.inference_url,
        socket_pool_size=1,
    ) as client:
        oversized_request = await client.continue_from_raw_prompt(
            ContinueFromRawPromptParams(
                max_tokens=1,
                raw_prompt="x" * BALANCER_MAX_WEBSOCKET_FRAME_BYTES,
            )
        )

        with pytest.raises(ConnectionDroppedError):
            await anext(oversized_request)

        follow_up_answer = [
            message.error_code
            async for message in await client.continue_from_raw_prompt(
                ContinueFromRawPromptParams(max_tokens=1, raw_prompt="Hello")
            )
        ]

    assert follow_up_answer == [GATEWAY_TIMEOUT]
