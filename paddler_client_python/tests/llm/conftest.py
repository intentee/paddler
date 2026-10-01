from collections.abc import AsyncIterator

import pytest
from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.balancer_addresses import BalancerAddresses
from paddler_test_cluster.nomic_embed_text_v1_5_desired_state import (
    NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE,
)
from paddler_test_cluster.paddler_cluster import paddler_cluster
from paddler_test_cluster.qwen3_0_6b_desired_state import QWEN3_0_6B_DESIRED_STATE

QWEN3_AGENT = AgentSpec(name="qwen3-agent", slots=2)
NOMIC_EMBED_AGENT = AgentSpec(name="nomic-embed-agent", slots=1)


@pytest.fixture
async def qwen3_cluster() -> AsyncIterator[BalancerAddresses]:
    async with paddler_cluster(QWEN3_0_6B_DESIRED_STATE, [QWEN3_AGENT]) as addresses:
        yield addresses


@pytest.fixture
async def nomic_embed_cluster() -> AsyncIterator[BalancerAddresses]:
    async with paddler_cluster(
        NOMIC_EMBED_TEXT_V1_5_DESIRED_STATE, [NOMIC_EMBED_AGENT]
    ) as addresses:
        yield addresses
