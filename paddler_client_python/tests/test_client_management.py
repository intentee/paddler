import json

import httpx
import pytest

from paddler_client.client_management import ClientManagement
from paddler_client.error import HttpError
from paddler_client.types.balancer_desired_state import BalancerDesiredState


def _agent_snapshot_json() -> dict[str, object]:
    return {
        "desired_slots_total": 4,
        "download_current": 0,
        "download_filename": None,
        "download_total": 0,
        "id": "agent-1",
        "issues": [],
        "model_path": "/models/test.gguf",
        "name": "test-agent",
        "slots_processing": 2,
        "slots_total": 4,
        "state_application_status": "Applied",
        "uses_chat_template_override": False,
    }


async def test_get_health_returns_ok() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "http://test:8085/health"

        return httpx.Response(200, text="OK")

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_health()
        assert result == "OK"
    finally:
        await client.close()


async def test_get_health_raises_on_error() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(503, text="Unavailable")

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        with pytest.raises(HttpError) as exc_info:
            await client.get_health()
        assert exc_info.value.status_code == 503
    finally:
        await client.close()


async def test_get_agents_deserializes_snapshot() -> None:
    response_data = {"agents": [_agent_snapshot_json()]}

    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "http://test:8085/api/v1/agents"

        return httpx.Response(200, json=response_data)

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_agents()
        assert len(result.agents) == 1
        assert result.agents[0].id == "agent-1"
        assert result.agents[0].slots_processing == 2
        assert result.agents[0].model_path == "/models/test.gguf"
    finally:
        await client.close()


async def test_get_balancer_desired_state_deserializes() -> None:
    response_data = {
        "chat_template_override": None,
        "inference_parameters": {
            "batch_n_tokens": 512,
            "context_size": 8192,
            "embedding_n_seq_max": 16,
            "enable_embeddings": False,
            "image_resize_to_fit": 1024,
            "min_p": 0.05,
            "penalty_frequency": 0.0,
            "penalty_last_n": -1,
            "penalty_presence": 0.8,
            "penalty_repeat": 1.1,
            "pooling_type": "Last",
            "temperature": 0.8,
            "top_k": 80,
            "top_p": 0.8,
        },
        "model": "None",
        "multimodal_projection": "None",
        "use_chat_template_override": False,
    }

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=response_data)

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_balancer_desired_state()
        assert result.model.variant == "None"
        assert result.inference_parameters.temperature == 0.8
    finally:
        await client.close()


async def test_put_balancer_desired_state_sends_json() -> None:
    received_body: list[bytes] = []

    def handler(request: httpx.Request) -> httpx.Response:
        received_body.append(request.content)

        return httpx.Response(200)

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        state = BalancerDesiredState()
        await client.put_balancer_desired_state(state)
        body = json.loads(received_body[0])
        assert body["model"] == "None"
        assert body["use_chat_template_override"] is False
    finally:
        await client.close()


async def test_get_buffered_requests_deserializes() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={"buffered_requests_current": 5},
        )

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_buffered_requests()
        assert result.buffered_requests_current == 5
    finally:
        await client.close()


async def test_get_chat_template_override_returns_none_for_null() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text="null")

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_chat_template_override("agent-1")
        assert result is None
    finally:
        await client.close()


async def test_get_chat_template_override_returns_template() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={"content": "{{ messages }}"},
        )

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_chat_template_override("agent-1")
        assert result is not None
        assert result.content == "{{ messages }}"
    finally:
        await client.close()


async def test_get_model_metadata_returns_none_for_null() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text="null")

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_model_metadata("agent-1")
        assert result is None
    finally:
        await client.close()


async def test_get_model_metadata_returns_metadata() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={"metadata": {"arch": "llama"}},
        )

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_model_metadata("agent-1")
        assert result is not None
        assert result.metadata["arch"] == "llama"
    finally:
        await client.close()


async def test_get_metrics_returns_text() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "http://test:8085/metrics"

        return httpx.Response(
            200,
            text="paddler_requests_total 42",
        )

    transport = httpx.MockTransport(handler)
    client = ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_metrics()
        assert "paddler_requests_total" in result
    finally:
        await client.close()


async def test_context_manager() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text="OK")

    transport = httpx.MockTransport(handler)

    async with ClientManagement(
        url="http://test:8085",
        http_client=httpx.AsyncClient(transport=transport),
    ) as client:
        result = await client.get_health()
        assert result == "OK"
