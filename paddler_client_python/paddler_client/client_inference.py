from __future__ import annotations

import uuid
from typing import TYPE_CHECKING, Literal, Self

import httpx

from paddler_client.error import HttpError
from paddler_client.inference_socket_pool import InferenceSocketPool
from paddler_client.inference_socket_url import inference_socket_url
from paddler_client.stream_ndjson import stream_ndjson_inference_messages

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from pydantic import BaseModel

    from paddler_client.inference_message import InferenceMessage
    from paddler_client.inference_socket_connection import ResponseStream
    from paddler_client.types.continue_from_conversation_history_params import (
        ContinueFromConversationHistoryParams,
    )
    from paddler_client.types.continue_from_raw_prompt_params import (
        ContinueFromRawPromptParams,
    )
    from paddler_client.types.generate_embedding_batch_params import (
        GenerateEmbeddingBatchParams,
    )

InferenceRequestVariant = Literal[
    "ContinueFromConversationHistory",
    "ContinueFromRawPrompt",
]


class ClientInference:
    def __init__(
        self,
        url: str,
        socket_pool_size: int = 4,
        http_client: httpx.AsyncClient | None = None,
    ) -> None:
        self._url = url.rstrip("/")
        self._socket_pool_size = socket_pool_size
        self._http_client = (
            http_client if http_client is not None else httpx.AsyncClient()
        )
        self._socket_pool: InferenceSocketPool | None = None

    def _get_socket_pool(self) -> InferenceSocketPool:
        if self._socket_pool is None:
            ws_url = inference_socket_url(self._url)
            self._socket_pool = InferenceSocketPool(
                url=ws_url,
                pool_size=self._socket_pool_size,
            )

        return self._socket_pool

    async def get_health(self) -> str:
        url = f"{self._url}/health"
        response = await self._http_client.get(url)

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return response.text

    async def continue_from_conversation_history(
        self,
        params: ContinueFromConversationHistoryParams,
    ) -> ResponseStream:
        return await self._send_ws_request(
            "ContinueFromConversationHistory",
            params,
        )

    async def continue_from_raw_prompt(
        self,
        params: ContinueFromRawPromptParams,
    ) -> ResponseStream:
        return await self._send_ws_request(
            "ContinueFromRawPrompt",
            params,
        )

    async def post_continue_from_conversation_history(
        self,
        params: ContinueFromConversationHistoryParams,
    ) -> AsyncIterator[InferenceMessage]:
        async for message in self._stream_ndjson_post(
            "/api/v1/continue_from_conversation_history",
            params,
        ):
            yield message

    async def generate_embedding_batch(
        self,
        params: GenerateEmbeddingBatchParams,
    ) -> AsyncIterator[InferenceMessage]:
        async for message in self._stream_ndjson_post(
            "/api/v1/generate_embedding_batch",
            params,
        ):
            yield message

    async def close(self) -> None:
        if self._socket_pool is not None:
            await self._socket_pool.close()

        await self._http_client.aclose()

    async def __aenter__(self) -> Self:
        return self

    async def __aexit__(self, *args: object) -> None:
        await self.close()

    async def _send_ws_request(
        self,
        variant: InferenceRequestVariant,
        params: BaseModel,
    ) -> ResponseStream:
        request_id = str(uuid.uuid4())
        message: dict[str, object] = {
            "Request": {
                "id": request_id,
                "request": {
                    variant: params.model_dump(
                        mode="json",
                        exclude_none=True,
                        by_alias=True,
                    ),
                },
            },
        }
        pool = self._get_socket_pool()

        return await pool.send_request(request_id, message)

    async def _stream_ndjson_post(
        self,
        path: str,
        params: BaseModel,
    ) -> AsyncIterator[InferenceMessage]:
        url = f"{self._url}{path}"
        request_body = params.model_dump(
            mode="json",
            exclude_none=True,
            by_alias=True,
        )

        async with self._http_client.stream(
            "POST",
            url,
            json=request_body,
            headers={"Content-Type": "application/json"},
        ) as response:
            async for message in stream_ndjson_inference_messages(response):
                yield message
