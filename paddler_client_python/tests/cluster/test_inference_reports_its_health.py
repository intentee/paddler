from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference


async def test_inference_reports_its_health(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientInference(url=cluster_without_agents.inference_url) as client:
        assert await client.get_health() == "OK"
