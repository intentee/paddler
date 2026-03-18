from __future__ import annotations

import httpx

from paddler_client.error import HttpError


async def raise_for_streaming_error(response: httpx.Response) -> None:
    if not response.is_success:
        body = await response.aread()

        raise HttpError(
            response.status_code,
            body.decode("utf-8", errors="replace"),
        )
