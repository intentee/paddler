from contextlib import aclosing

from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement


async def test_management_streams_buffered_request_snapshots(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with (
        ClientManagement(url=cluster_without_agents.management_url) as client,
        aclosing(client.buffered_requests_stream()) as snapshots,
    ):
        snapshot = await anext(snapshots)

    assert snapshot.buffered_requests_current == 0
