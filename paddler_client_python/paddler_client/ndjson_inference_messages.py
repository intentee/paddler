from __future__ import annotations

import json
from typing import TYPE_CHECKING

from paddler_client.inference_message import (
    InferenceMessage,
    parse_inference_client_message,
)

if TYPE_CHECKING:
    from collections.abc import AsyncGenerator, AsyncIterable


async def ndjson_inference_messages(
    lines: AsyncIterable[str],
) -> AsyncGenerator[InferenceMessage, None]:
    async for line in lines:
        stripped_line = line.strip()

        if stripped_line:
            yield parse_inference_client_message(json.loads(stripped_line))
