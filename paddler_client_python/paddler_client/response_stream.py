from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import asyncio

    from paddler_client.inference_message import InferenceMessage


class ResponseStream:
    def __init__(
        self,
        queue: asyncio.Queue[InferenceMessage | Exception],
    ) -> None:
        self._queue = queue
        self._done = False

    def __aiter__(self) -> ResponseStream:
        return self

    async def __anext__(self) -> InferenceMessage:
        if self._done:
            raise StopAsyncIteration

        item = await self._queue.get()

        if isinstance(item, Exception):
            self._done = True
            raise item

        if item.is_terminal:
            self._done = True

        return item
