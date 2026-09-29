import pytest
from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.error import ConnectionDroppedError
from paddler_client.inference_socket_connection import InferenceSocketConnection
from paddler_client.inference_socket_url import inference_socket_url


async def test_inference_socket_connection_rejects_requests_after_it_closes(
    cluster_without_agents: BalancerAddresses,
) -> None:
    connection = await InferenceSocketConnection.connect(
        inference_socket_url(cluster_without_agents.inference_url)
    )

    await connection.close()

    with pytest.raises(ConnectionDroppedError) as connection_dropped:
        await connection.send("request-after-close", "{}")

    assert connection_dropped.value.request_id == "request-after-close"
