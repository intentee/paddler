import asyncio
from collections.abc import AsyncIterator, Sequence
from contextlib import aclosing, asynccontextmanager
from datetime import timedelta
from typing import TYPE_CHECKING

from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_management import ClientManagement

from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.balancer_addresses import BalancerAddresses
from paddler_test_cluster.observation_window import MODEL_LOAD
from paddler_test_cluster.spawned_agent import spawn_agent
from paddler_test_cluster.spawned_balancer import (
    PADDLER_BUFFERED_REQUEST_TIMEOUT,
    SpawnedBalancer,
)
from paddler_test_cluster.wait_for_agent_ready import wait_for_agent_ready

if TYPE_CHECKING:
    from paddler_test_cluster.spawned_process import SpawnedProcess


@asynccontextmanager
async def paddler_cluster(
    desired_state: BalancerDesiredState,
    agents: Sequence[AgentSpec],
    *,
    buffered_request_timeout: timedelta = PADDLER_BUFFERED_REQUEST_TIMEOUT,
) -> AsyncIterator[BalancerAddresses]:
    balancer = await SpawnedBalancer.spawn(
        buffered_request_timeout=buffered_request_timeout,
    )
    agent_processes: list[SpawnedProcess] = []
    expects_loaded_slots = desired_state.model != AgentDesiredModel.none()

    try:
        async with ClientManagement(url=balancer.addresses.management_url) as client:
            await client.put_balancer_desired_state(desired_state)

            async with (
                aclosing(client.agents_stream()) as snapshots,
                asyncio.timeout(MODEL_LOAD.total_seconds()),
            ):
                for agent in agents:
                    agent_processes.append(
                        await spawn_agent(balancer.addresses.management, agent)
                    )

                    await wait_for_agent_ready(
                        snapshots,
                        agent.name,
                        agent.slots if expects_loaded_slots else 0,
                    )

        yield balancer.addresses
    finally:
        for agent_process in agent_processes:
            await agent_process.terminate()

        await balancer.process.terminate()
