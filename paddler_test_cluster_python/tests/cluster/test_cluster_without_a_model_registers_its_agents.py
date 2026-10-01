from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_management import ClientManagement

from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.paddler_cluster import paddler_cluster


async def test_cluster_without_a_model_registers_its_agents() -> None:
    async with (
        paddler_cluster(
            BalancerDesiredState(),
            [AgentSpec(name="first", slots=1), AgentSpec(name="second", slots=2)],
        ) as addresses,
        ClientManagement(url=addresses.management_url) as client,
    ):
        snapshot = await client.get_agents()

    assert {agent.name for agent in snapshot.agents} == {"first", "second"}
