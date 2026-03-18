from __future__ import annotations

import asyncio
import contextlib
import json
import logging

import websockets
from websockets.asyncio.client import ClientConnection, connect

from paddler_client.error import ConnectionDroppedError, JsonError
from paddler_client.inference_message import (
    InferenceMessage,
    parse_inference_client_message,
)

logger = logging.getLogger(__name__)


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
            raise item

        if item.is_terminal:
            self._done = True

        return item


class InferenceSocketConnection:
    def __init__(self, url: str) -> None:
        self._url = url
        self._ws: ClientConnection | None = None
        self._pending: dict[
            str, asyncio.Queue[InferenceMessage | Exception]
        ] = {}
        self._write_queue: asyncio.Queue[str] = asyncio.Queue()
        self._read_task: asyncio.Task[None] | None = None
        self._write_task: asyncio.Task[None] | None = None
        self._connected = False

    @property
    def is_connected(self) -> bool:
        return self._connected

    async def connect(self) -> None:
        self._ws = await connect(self._url)
        self._connected = True
        self._read_task = asyncio.create_task(self._read_loop())
        self._write_task = asyncio.create_task(self._write_loop())

    async def send(
        self,
        request_id: str,
        json_message: str,
    ) -> ResponseStream:
        response_queue: asyncio.Queue[InferenceMessage | Exception] = (
            asyncio.Queue()
        )
        self._pending[request_id] = response_queue

        try:
            await self._write_queue.put(json_message)
        except Exception:  # noqa: BLE001
            del self._pending[request_id]
            raise ConnectionDroppedError(request_id) from None

        return ResponseStream(response_queue)

    async def close(self) -> None:
        self._connected = False

        if self._ws is not None:
            await self._ws.close()

        if self._write_task is not None:
            self._write_task.cancel()

            with contextlib.suppress(asyncio.CancelledError):
                await self._write_task

        if self._read_task is not None:
            self._read_task.cancel()

            with contextlib.suppress(asyncio.CancelledError):
                await self._read_task

    async def _read_loop(self) -> None:
        try:
            if self._ws is None:
                return

            async for raw_message in self._ws:
                if not isinstance(raw_message, str):
                    logger.warning(
                        "Received unexpected binary WebSocket message"
                    )
                    continue

                try:
                    data = json.loads(raw_message)
                    message = parse_inference_client_message(data)
                except Exception:
                    logger.exception("Failed to parse WebSocket message")
                    self._push_parse_error_to_pending(raw_message)

                    continue

                queue = self._pending.get(message.request_id)

                if queue is None:
                    logger.warning(
                        "Received message for unknown request: %s",
                        message.request_id,
                    )
                    continue

                queue.put_nowait(message)

                if message.is_terminal:
                    del self._pending[message.request_id]
        except asyncio.CancelledError:
            pass
        except websockets.ConnectionClosed:
            logger.debug("WebSocket connection closed")
        except Exception:
            logger.exception("WebSocket read error")
        finally:
            self._connected = False

            for request_id, queue in self._pending.items():
                queue.put_nowait(ConnectionDroppedError(request_id))

            self._pending.clear()

    def _push_parse_error_to_pending(self, raw_message: str) -> None:
        try:
            data = json.loads(raw_message)

            if isinstance(data, dict):
                request_id = None

                if "Response" in data:
                    request_id = data["Response"].get("request_id")
                elif "Error" in data:
                    request_id = data["Error"].get("request_id")

                if request_id and request_id in self._pending:
                    self._pending[request_id].put_nowait(
                        JsonError("Failed to parse message", raw_data=raw_message),
                    )
                    del self._pending[request_id]
        except (json.JSONDecodeError, KeyError, TypeError):
            pass

    async def _write_loop(self) -> None:
        try:
            while self._connected:
                message = await self._write_queue.get()

                if self._ws is not None:
                    await self._ws.send(message)
        except asyncio.CancelledError:
            pass
        except websockets.ConnectionClosed:
            logger.debug("WebSocket write connection closed")
        except Exception:
            logger.exception("WebSocket write error")
        finally:
            self._connected = False
