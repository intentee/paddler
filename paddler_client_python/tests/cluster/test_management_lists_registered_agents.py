from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)
from paddler_client.agent_status import AgentStatus
from paddler_client.client_management import ClientManagement
from paddler_client.model_download_status import ModelDownloadStatus
from tests.cluster.conftest import MODEL_LESS_AGENT


async def test_management_lists_registered_agents(
    cluster_with_a_model_less_agent: BalancerAddresses,
) -> None:
    async with ClientManagement(
        url=cluster_with_a_model_less_agent.management_url
    ) as client:
        snapshot = await client.get_agents()

    assert [agent.name for agent in snapshot.agents] == [MODEL_LESS_AGENT.name]
    assert [agent.status for agent in snapshot.agents] == [
        AgentStatus(
            desired_slots_total=MODEL_LESS_AGENT.slots,
            download_status=ModelDownloadStatus(variant="NotDownloading"),
            issues=[],
            model_path=None,
            slots_total=0,
            state_application_status=AgentStateApplicationStatus.APPLIED,
            uses_chat_template_override=False,
        )
    ]
