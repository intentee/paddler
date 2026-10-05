from contextlib import aclosing

from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement
from tests.cluster.conftest import MODEL_LESS_AGENT


async def test_management_streams_agent_snapshots(
    cluster_with_a_model_less_agent: BalancerAddresses,
) -> None:
    async with (
        ClientManagement(url=cluster_with_a_model_less_agent.management_url) as client,
        aclosing(client.agents_stream()) as snapshots,
    ):
        snapshot = await anext(snapshots)

    assert [agent.name for agent in snapshot.agents] == [MODEL_LESS_AGENT.name]
