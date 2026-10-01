from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement


async def test_management_reports_its_health(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientManagement(url=cluster_without_agents.management_url) as client:
        assert await client.get_health() == "OK"
