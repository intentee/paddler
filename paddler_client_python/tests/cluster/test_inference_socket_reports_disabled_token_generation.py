import json

from paddler_test_cluster.paddler_cluster import paddler_cluster
from websockets.asyncio.client import connect

from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.inference_message import InferenceMessageKind
from paddler_client.inference_parameters import InferenceParameters
from paddler_client.inference_socket_url import inference_socket_url


async def wait_until_token_generation_is_disabled(inference_url: str) -> None:
    async with connect(inference_socket_url(inference_url)) as websocket:
        assert json.loads(await websocket.recv()) == {
            "Notification": "TokenGenerationDisabled"
        }


async def test_inference_socket_reports_disabled_token_generation() -> None:
    async with (
        paddler_cluster(
            BalancerDesiredState(
                inference_parameters=InferenceParameters(enable_embeddings=True)
            ),
            [],
        ) as addresses,
        ClientInference(url=addresses.inference_url, socket_pool_size=1) as client,
    ):
        await wait_until_token_generation_is_disabled(addresses.inference_url)

        kinds = [
            message.kind
            async for message in await client.continue_from_raw_prompt(
                ContinueFromRawPromptParams(max_tokens=1, raw_prompt="Hello")
            )
        ]

    assert kinds == [InferenceMessageKind.TOKEN_GENERATION_DISABLED]
