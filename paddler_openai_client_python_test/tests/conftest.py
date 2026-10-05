from collections.abc import AsyncIterator

import pytest
from openai import OpenAI
from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.paddler_cluster import paddler_cluster
from paddler_test_cluster.qwen3_0_6b_desired_state import QWEN3_0_6B_DESIRED_STATE

QWEN3_AGENT = AgentSpec(name="qwen3-agent", slots=1)


@pytest.fixture
def model() -> str:
    return "qwen3"


@pytest.fixture
async def openai_client() -> AsyncIterator[OpenAI]:
    async with paddler_cluster(QWEN3_0_6B_DESIRED_STATE, [QWEN3_AGENT]) as addresses:
        with OpenAI(
            base_url=f"{addresses.compat_openai_url}/v1", api_key="paddler"
        ) as openai_client:
            yield openai_client
