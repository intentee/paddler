from __future__ import annotations

from typing import TYPE_CHECKING

from paddler_client.ndjson_inference_messages import ndjson_inference_messages
from paddler_client.raise_for_streaming_error import raise_for_streaming_error

if TYPE_CHECKING:
    from collections.abc import AsyncGenerator

    import httpx

    from paddler_client.inference_message import InferenceMessage


async def stream_ndjson_inference_messages(
    response: httpx.Response,
) -> AsyncGenerator[InferenceMessage, None]:
    await raise_for_streaming_error(response)

    async for message in ndjson_inference_messages(response.aiter_lines()):
        yield message
