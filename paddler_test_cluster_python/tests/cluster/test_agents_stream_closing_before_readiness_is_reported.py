import pytest
from paddler_client.client_management import ClientManagement

from paddler_test_cluster.error import AgentsStreamClosedError
from paddler_test_cluster.spawned_balancer import SpawnedBalancer
from paddler_test_cluster.wait_for_agent_ready import wait_for_agent_ready


async def test_agents_stream_closing_before_readiness_is_reported() -> None:
    balancer = await SpawnedBalancer.spawn()

    async with ClientManagement(url=balancer.addresses.management_url) as client:
        snapshots = client.agents_stream()

        await anext(snapshots)
        await balancer.process.terminate()

        with pytest.raises(AgentsStreamClosedError) as stream_closed:
            await wait_for_agent_ready(snapshots, "agent-that-never-joined", 1)

    assert stream_closed.value.agent_name == "agent-that-never-joined"
