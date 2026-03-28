from __future__ import annotations

import asyncio
import json
import logging

from paddler_client.error import ConnectionDroppedError
from paddler_client.inference_socket_connection import (
    InferenceSocketConnection,
    ResponseStream,
)

logger = logging.getLogger(__name__)


class InferenceSocketPool:
    def __init__(self, url: str, pool_size: int) -> None:
        if pool_size < 1:
            raise ValueError(f"pool_size must be >= 1, got {pool_size}")

        self._url = url
        self._pool_size = pool_size
        self._connections: list[InferenceSocketConnection | None] = [None] * pool_size
        self._next_idx = 0
        self._lock = asyncio.Lock()

    async def send_request(
        self,
        request_id: str,
        message: dict[str, object],
    ) -> ResponseStream:
        json_str = json.dumps(message)

        async with self._lock:
            idx = self._next_idx
            self._next_idx = (self._next_idx + 1) % self._pool_size
            connection = await self._ensure_connected(idx)

        try:
            return await connection.send(request_id, json_str)
        except ConnectionDroppedError:
            logger.info("Connection dropped, reconnecting...")

            async with self._lock:
                connection = await self._ensure_connected(idx, force=True)

            return await connection.send(request_id, json_str)

    async def close(self) -> None:
        for connection in self._connections:
            if connection is not None:
                await connection.close()

        self._connections = [None] * self._pool_size

    async def _ensure_connected(
        self,
        idx: int,
        *,
        force: bool = False,
    ) -> InferenceSocketConnection:
        connection = self._connections[idx]

        if not force and connection is not None and connection.is_connected:
            return connection

        if connection is not None:
            await connection.close()

        new_connection = InferenceSocketConnection(self._url)
        await new_connection.connect()
        self._connections[idx] = new_connection

        return new_connection
