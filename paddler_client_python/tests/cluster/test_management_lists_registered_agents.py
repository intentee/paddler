from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement
from tests.cluster.conftest import MODEL_LESS_AGENT


async def test_management_lists_registered_agents(
    cluster_with_a_model_less_agent: BalancerAddresses,
) -> None:
    async with ClientManagement(
        url=cluster_with_a_model_less_agent.management_url
    ) as client:
        snapshot = await client.get_agents()

    assert [
        (agent.name, agent.desired_slots_total, agent.slots_total)
        for agent in snapshot.agents
    ] == [(MODEL_LESS_AGENT.name, MODEL_LESS_AGENT.slots, 0)]
