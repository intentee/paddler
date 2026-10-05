from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.inference_message import InferenceMessageKind

MAX_TOKENS = 8


async def test_raw_prompt_streams_tokens_over_the_inference_socket(
    qwen3_cluster: BalancerAddresses,
) -> None:
    async with ClientInference(url=qwen3_cluster.inference_url) as client:
        messages = [
            message
            async for message in await client.continue_from_raw_prompt(
                ContinueFromRawPromptParams(
                    max_tokens=MAX_TOKENS,
                    raw_prompt="The capital of France is",
                )
            )
        ]

    *tokens, done = messages

    assert all(token.is_token for token in tokens)
    assert done.kind == InferenceMessageKind.DONE
    assert len(tokens) == MAX_TOKENS
