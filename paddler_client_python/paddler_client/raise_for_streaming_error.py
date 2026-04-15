from __future__ import annotations

from typing import TYPE_CHECKING

from paddler_client.error import HttpError

if TYPE_CHECKING:
    import httpx


async def raise_for_streaming_error(response: httpx.Response) -> None:
    if not response.is_success:
        body = await response.aread()

        raise HttpError(
            response.status_code,
            body.decode("utf-8", errors="replace"),
        )
