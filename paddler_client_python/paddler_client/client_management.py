from __future__ import annotations

from typing import TYPE_CHECKING, Self

import httpx

from paddler_client.agent_controller_pool_snapshot import (
    AgentControllerPoolSnapshot,
)
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.buffered_request_manager_snapshot import (
    BufferedRequestManagerSnapshot,
)
from paddler_client.chat_template import ChatTemplate
from paddler_client.error import HttpError
from paddler_client.model_metadata import ModelMetadata
from paddler_client.stream_sse import stream_sse

if TYPE_CHECKING:
    from collections.abc import AsyncIterator


class ClientManagement:
    def __init__(
        self,
        url: str,
        http_client: httpx.AsyncClient | None = None,
    ) -> None:
        self._url = url.rstrip("/")
        self._http_client = (
            http_client if http_client is not None else httpx.AsyncClient()
        )

    async def get_health(self) -> str:
        response = await self._http_client.get(f"{self._url}/health")

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return response.text

    async def get_agents(self) -> AgentControllerPoolSnapshot:
        response = await self._http_client.get(
            f"{self._url}/api/v1/agents",
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return AgentControllerPoolSnapshot.model_validate_json(
            response.content,
        )

    async def agents_stream(
        self,
    ) -> AsyncIterator[AgentControllerPoolSnapshot]:
        url = f"{self._url}/api/v1/agents/stream"

        async with self._http_client.stream("GET", url) as response:
            async for data in stream_sse(response):
                yield AgentControllerPoolSnapshot.model_validate_json(data)

    async def get_balancer_desired_state(self) -> BalancerDesiredState:
        response = await self._http_client.get(
            f"{self._url}/api/v1/balancer_desired_state",
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return BalancerDesiredState.model_validate_json(response.content)

    async def put_balancer_desired_state(
        self,
        state: BalancerDesiredState,
    ) -> None:
        response = await self._http_client.put(
            f"{self._url}/api/v1/balancer_desired_state",
            content=state.model_dump_json(
                exclude_none=True,
                by_alias=True,
            ),
            headers={"Content-Type": "application/json"},
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

    async def get_buffered_requests(
        self,
    ) -> BufferedRequestManagerSnapshot:
        response = await self._http_client.get(
            f"{self._url}/api/v1/buffered_requests",
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return BufferedRequestManagerSnapshot.model_validate_json(
            response.content,
        )

    async def buffered_requests_stream(
        self,
    ) -> AsyncIterator[BufferedRequestManagerSnapshot]:
        url = f"{self._url}/api/v1/buffered_requests/stream"

        async with self._http_client.stream("GET", url) as response:
            async for data in stream_sse(response):
                yield BufferedRequestManagerSnapshot.model_validate_json(
                    data,
                )

    async def get_chat_template_override(
        self,
        agent_id: str,
    ) -> ChatTemplate | None:
        response = await self._http_client.get(
            f"{self._url}/api/v1/agent/{agent_id}/chat_template_override",
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        if not response.content or response.text == "null":
            return None

        return ChatTemplate.model_validate_json(response.content)

    async def get_model_metadata(
        self,
        agent_id: str,
    ) -> ModelMetadata | None:
        response = await self._http_client.get(
            f"{self._url}/api/v1/agent/{agent_id}/model_metadata",
        )

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        if not response.content or response.text == "null":
            return None

        return ModelMetadata.model_validate_json(response.content)

    async def get_metrics(self) -> str:
        response = await self._http_client.get(f"{self._url}/metrics")

        if not response.is_success:
            raise HttpError(response.status_code, response.text)

        return response.text

    async def close(self) -> None:
        await self._http_client.aclose()

    async def __aenter__(self) -> Self:
        return self

    async def __aexit__(self, *args: object) -> None:
        await self.close()
