from collections.abc import AsyncIterator

from paddler_client.agent_controller_pool_snapshot import (
    AgentControllerPoolSnapshot,
)
from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)

from paddler_test_cluster.error import (
    AgentReportedIssuesError,
    AgentsStreamClosedError,
)


async def wait_for_agent_ready(
    snapshots: AsyncIterator[AgentControllerPoolSnapshot],
    agent_name: str,
    expected_slots_total: int,
) -> None:
    async for snapshot in snapshots:
        for agent in (agent for agent in snapshot.agents if agent.name == agent_name):
            if agent.status.issues:
                raise AgentReportedIssuesError(agent_name, agent.status.issues)

            if (
                agent.status.state_application_status
                == AgentStateApplicationStatus.APPLIED
                and agent.status.slots_total == expected_slots_total
            ):
                return

    raise AgentsStreamClosedError(agent_name)
