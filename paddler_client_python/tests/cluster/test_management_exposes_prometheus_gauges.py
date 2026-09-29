from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_management import ClientManagement

IDLE_CLUSTER_METRICS = """\
# HELP paddler_slots_processing Number of processing slots
# TYPE paddler_slots_processing gauge
paddler_slots_processing 0

# HELP paddler_slots_total Number of total slots
# TYPE paddler_slots_total gauge
paddler_slots_total 0

# HELP paddler_requests_buffered Number of buffered requests
# TYPE paddler_requests_buffered gauge
paddler_requests_buffered 0
"""


async def test_management_exposes_prometheus_gauges(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientManagement(url=cluster_without_agents.management_url) as client:
        assert await client.get_metrics() == IDLE_CLUSTER_METRICS
