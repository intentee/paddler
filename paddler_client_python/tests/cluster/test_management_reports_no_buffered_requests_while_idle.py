from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement


async def test_management_reports_no_buffered_requests_while_idle(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientManagement(url=cluster_without_agents.management_url) as client:
        snapshot = await client.get_buffered_requests()

    assert snapshot.buffered_requests_current == 0
