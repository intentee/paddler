from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_management import ClientManagement


async def test_management_serves_the_desired_state_it_stores(
    cluster_without_agents: BalancerAddresses,
) -> None:
    desired_state = BalancerDesiredState(
        model=AgentDesiredModel.local_to_agent("/models/stored.gguf"),
    )

    async with ClientManagement(url=cluster_without_agents.management_url) as client:
        await client.put_balancer_desired_state(desired_state)

        assert await client.get_balancer_desired_state() == desired_state
