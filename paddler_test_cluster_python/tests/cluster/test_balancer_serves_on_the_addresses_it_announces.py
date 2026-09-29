import httpx
from paddler_client.client_inference import ClientInference
from paddler_client.client_management import ClientManagement

from paddler_test_cluster.spawned_balancer import SpawnedBalancer


async def test_balancer_serves_on_the_addresses_it_announces() -> None:
    balancer = await SpawnedBalancer.spawn()

    try:
        async with ClientManagement(url=balancer.addresses.management_url) as client:
            assert await client.get_health() == "OK"

        async with ClientInference(url=balancer.addresses.inference_url) as client:
            assert await client.get_health() == "OK"

        async with httpx.AsyncClient() as http_client:
            compat_openai_health = await http_client.get(
                f"{balancer.addresses.compat_openai_url}/health"
            )

        assert compat_openai_health.text == "OK"
    finally:
        await balancer.process.terminate()
