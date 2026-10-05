from __future__ import annotations

import pytest

from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)
from paddler_client.client_management import ClientManagement

pytestmark = pytest.mark.integration


async def test_health(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        result = await client.get_health()

    assert result == "OK"


async def test_get_agents(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        snapshot = await client.get_agents()

    assert isinstance(snapshot.agents, list)

    for agent in snapshot.agents:
        assert agent.id
        assert agent.slots_total >= 0
        assert agent.state_application_status in (
            AgentStateApplicationStatus.APPLIED,
            AgentStateApplicationStatus.FRESH,
            AgentStateApplicationStatus.STUCK,
            AgentStateApplicationStatus.ATTEMPTED_AND_NOT_APPLIABLE,
        )


async def test_get_balancer_desired_state(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        state = await client.get_balancer_desired_state()

    assert state.inference_parameters is not None
    assert state.inference_parameters.temperature >= 0


async def test_get_buffered_requests(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        snapshot = await client.get_buffered_requests()

    assert snapshot.buffered_requests_current >= 0


async def test_get_metrics(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        metrics = await client.get_metrics()

    assert isinstance(metrics, str)
    assert len(metrics) > 0


async def test_put_and_get_balancer_desired_state_roundtrip(
    management_url: str,
) -> None:
    async with ClientManagement(url=management_url) as client:
        original = await client.get_balancer_desired_state()
        await client.put_balancer_desired_state(original)
        restored = await client.get_balancer_desired_state()

    assert (
        restored.inference_parameters.temperature
        == original.inference_parameters.temperature
    )


async def test_agents_stream_yields_snapshot(management_url: str) -> None:
    async with ClientManagement(url=management_url) as client:
        async for snapshot in client.agents_stream():
            assert isinstance(snapshot.agents, list)

            break
