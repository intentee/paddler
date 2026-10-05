from paddler_test_cluster.paddler_cluster import paddler_cluster

from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.chat_template import ChatTemplate
from paddler_client.client_management import ClientManagement
from tests.cluster.conftest import MODEL_LESS_AGENT

CHAT_TEMPLATE = ChatTemplate(content="{{ messages[0].content }}")


async def test_management_reports_the_chat_template_override_an_agent_applies() -> None:
    async with (
        paddler_cluster(
            BalancerDesiredState(
                chat_template_override=CHAT_TEMPLATE,
                use_chat_template_override=True,
            ),
            [MODEL_LESS_AGENT],
        ) as addresses,
        ClientManagement(url=addresses.management_url) as client,
    ):
        [agent] = (await client.get_agents()).agents

        assert await client.get_chat_template_override(agent.id) == CHAT_TEMPLATE
