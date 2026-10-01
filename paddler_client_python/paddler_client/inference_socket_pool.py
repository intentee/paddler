from __future__ import annotations

import asyncio
import json
from typing import TYPE_CHECKING

from paddler_client.error import InvalidSocketPoolSizeError
from paddler_client.inference_socket_connection import InferenceSocketConnection

if TYPE_CHECKING:
    from paddler_client.response_stream import ResponseStream


class InferenceSocketPool:
    def __init__(self, url: str, pool_size: int) -> None:
        if pool_size < 1:
            raise InvalidSocketPoolSizeError(pool_size)

        self._url = url
        self._pool_size = pool_size
        self._connections: list[InferenceSocketConnection | None] = [None] * pool_size
        self._next_connection_index = 0
        self._lock = asyncio.Lock()

    async def send_request(
        self,
        request_id: str,
        message: dict[str, object],
    ) -> ResponseStream:
        serialized_message = json.dumps(message)

        async with self._lock:
            connection_index = self._next_connection_index
            self._next_connection_index = (
                self._next_connection_index + 1
            ) % self._pool_size
            connection = await self._ensure_connected(connection_index)

        return await connection.send(request_id, serialized_message)

    async def close(self) -> None:
        for connection in self._connections:
            if connection is not None:
                await connection.close()

        self._connections = [None] * self._pool_size

    async def _ensure_connected(
        self, connection_index: int
    ) -> InferenceSocketConnection:
        connection = self._connections[connection_index]

        if connection is not None and connection.is_connected:
            return connection

        if connection is not None:
            await connection.close()

        new_connection = await InferenceSocketConnection.connect(self._url)
        self._connections[connection_index] = new_connection

        return new_connection
