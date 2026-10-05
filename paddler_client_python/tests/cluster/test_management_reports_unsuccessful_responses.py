from contextlib import aclosing
from typing import TYPE_CHECKING

import pytest
from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_management import ClientManagement
from paddler_client.error import HttpError

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

NOT_FOUND = 404


async def test_management_reports_unsuccessful_responses(
    cluster_without_agents: BalancerAddresses,
) -> None:
    async with ClientManagement(
        url=f"{cluster_without_agents.management_url}/not-a-paddler-route"
    ) as client:

        async def first_agents_snapshot() -> object:
            async with aclosing(client.agents_stream()) as snapshots:
                return await anext(snapshots)

        async def first_buffered_requests_snapshot() -> object:
            async with aclosing(client.buffered_requests_stream()) as snapshots:
                return await anext(snapshots)

        requests: list[Callable[[], Awaitable[object]]] = [
            client.get_health,
            client.get_agents,
            first_agents_snapshot,
            client.get_balancer_desired_state,
            lambda: client.put_balancer_desired_state(BalancerDesiredState()),
            client.get_buffered_requests,
            first_buffered_requests_snapshot,
            lambda: client.get_chat_template_override("agent-id"),
            lambda: client.get_model_metadata("agent-id"),
            client.get_metrics,
        ]

        for request in requests:
            with pytest.raises(HttpError) as unsuccessful:
                await request()

            assert unsuccessful.value.status_code == NOT_FOUND
