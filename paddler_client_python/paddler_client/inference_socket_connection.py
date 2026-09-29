from __future__ import annotations

import asyncio
import contextlib
import logging
from typing import TYPE_CHECKING, Self

import websockets
from websockets.asyncio.client import ClientConnection, connect

from paddler_client.error import ConnectionDroppedError
from paddler_client.pending_requests import PendingRequests

if TYPE_CHECKING:
    from paddler_client.response_stream import ResponseStream

logger = logging.getLogger(__name__)


class InferenceSocketConnection:
    def __init__(self, websocket: ClientConnection) -> None:
        self._websocket = websocket
        self._pending = PendingRequests()
        self._write_queue: asyncio.Queue[str] = asyncio.Queue()
        self._connected = True
        self._read_task = asyncio.create_task(self._read_loop())
        self._write_task = asyncio.create_task(self._write_loop())

    @classmethod
    async def connect(cls, url: str) -> Self:
        return cls(await connect(url))

    @property
    def is_connected(self) -> bool:
        return self._connected

    async def send(
        self,
        request_id: str,
        json_message: str,
    ) -> ResponseStream:
        if not self._connected:
            raise ConnectionDroppedError(request_id)

        response_stream = self._pending.register(request_id)
        await self._write_queue.put(json_message)

        return response_stream

    async def close(self) -> None:
        self._connected = False

        await self._websocket.close()

        for task in (self._write_task, self._read_task):
            task.cancel()

            with contextlib.suppress(
                asyncio.CancelledError, websockets.ConnectionClosed
            ):
                await task

    async def _read_loop(self) -> None:
        try:
            while True:
                self._pending.dispatch(await self._websocket.recv(decode=True))
        except websockets.ConnectionClosed:
            logger.debug("WebSocket connection closed")
        finally:
            self._drop_connection()
            self._write_task.cancel()

    async def _write_loop(self) -> None:
        try:
            while True:
                await self._websocket.send(await self._write_queue.get())
        finally:
            self._drop_connection()
            self._read_task.cancel()

    def _drop_connection(self) -> None:
        self._connected = False
        self._pending.drain()
