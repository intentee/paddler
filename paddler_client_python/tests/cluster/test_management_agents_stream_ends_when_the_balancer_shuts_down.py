from contextlib import aclosing

from paddler_test_cluster.spawned_balancer import SpawnedBalancer

from paddler_client.client_management import ClientManagement


async def test_management_agents_stream_ends_when_the_balancer_shuts_down() -> None:
    balancer = await SpawnedBalancer.spawn()

    try:
        async with (
            ClientManagement(url=balancer.addresses.management_url) as client,
            aclosing(client.agents_stream()) as snapshots,
        ):
            await anext(snapshots)
            await balancer.process.terminate()

            remaining_snapshots = [snapshot async for snapshot in snapshots]
    finally:
        await balancer.process.terminate()

    assert remaining_snapshots == []
