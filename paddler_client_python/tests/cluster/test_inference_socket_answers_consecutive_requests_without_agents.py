from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)

GATEWAY_TIMEOUT = 504


async def test_inference_socket_answers_consecutive_requests_without_agents(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientInference(
        url=cluster_without_agents.inference_url,
        socket_pool_size=1,
    ) as client:
        answers = [
            [
                message.error_code
                async for message in await client.continue_from_raw_prompt(
                    ContinueFromRawPromptParams(max_tokens=1, raw_prompt=raw_prompt)
                )
            ]
            for raw_prompt in ("first", "second")
        ]

    assert answers == [[GATEWAY_TIMEOUT], [GATEWAY_TIMEOUT]]
