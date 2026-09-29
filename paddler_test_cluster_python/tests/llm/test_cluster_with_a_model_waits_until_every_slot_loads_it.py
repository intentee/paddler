import pytest
from paddler_client.balancer_desired_state import BalancerDesiredState
from paddler_client.client_management import ClientManagement

from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.nomic_embed_text_v1_5_desired_state import (
    NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE,
)
from paddler_test_cluster.paddler_cluster import paddler_cluster
from paddler_test_cluster.qwen3_0_6b_desired_state import QWEN3_0_6B_DESIRED_STATE

AGENT = AgentSpec(name="loaded", slots=2)


@pytest.mark.parametrize(
    "desired_state",
    [QWEN3_0_6B_DESIRED_STATE, NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE],
)
async def test_cluster_with_a_model_waits_until_every_slot_loads_it(
    desired_state: BalancerDesiredState,
) -> None:
    async with (
        paddler_cluster(desired_state, [AGENT]) as addresses,
        ClientManagement(url=addresses.management_url) as client,
    ):
        [agent] = (await client.get_agents()).agents

    assert agent.model_path is not None
    assert agent.slots_total == AGENT.slots
