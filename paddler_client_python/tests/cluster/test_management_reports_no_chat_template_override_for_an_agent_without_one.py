from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement


async def test_management_reports_no_chat_template_override_for_an_agent_without_one(
    cluster_with_a_model_less_agent: BalancerAddresses,
) -> None:
    async with ClientManagement(
        url=cluster_with_a_model_less_agent.management_url
    ) as client:
        [agent] = (await client.get_agents()).agents

        assert await client.get_chat_template_override(agent.id) is None
