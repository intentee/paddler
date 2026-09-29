from __future__ import annotations

import asyncio
import json
import logging

from paddler_client.error import ConnectionDroppedError, JsonError
from paddler_client.inference_message import (
    InferenceMessage,
    parse_inference_client_message,
)
from paddler_client.response_stream import ResponseStream
from paddler_client.unparsed_message_request_id import unparsed_message_request_id

logger = logging.getLogger(__name__)


class PendingRequests:
    def __init__(self) -> None:
        self._queues: dict[str, asyncio.Queue[InferenceMessage | Exception]] = {}

    def register(self, request_id: str) -> ResponseStream:
        queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
        self._queues[request_id] = queue

        return ResponseStream(queue)

    def dispatch(self, raw_message: str) -> None:
        try:
            data = json.loads(raw_message)
        except json.JSONDecodeError:
            logger.exception("Received a WebSocket message that is not JSON")

            return

        if isinstance(data, dict) and "Notification" in data:
            return

        try:
            message = parse_inference_client_message(data)
        except (KeyError, TypeError, ValueError):
            logger.exception("Failed to parse WebSocket message")
            self._fail(
                unparsed_message_request_id(data),
                JsonError("Failed to parse message", raw_data=raw_message),
            )

            return

        queue = self._queues.get(message.request_id)

        if queue is None:
            logger.warning(
                "Received message for unknown request: %s",
                message.request_id,
            )

            return

        queue.put_nowait(message)

        if message.is_terminal:
            del self._queues[message.request_id]

    def drain(self) -> None:
        queues = self._queues
        self._queues = {}

        for request_id, queue in queues.items():
            queue.put_nowait(ConnectionDroppedError(request_id))

    def _fail(self, request_id: str | None, error: Exception) -> None:
        queue = self._queues.pop(request_id, None) if request_id else None

        if queue is not None:
            queue.put_nowait(error)
