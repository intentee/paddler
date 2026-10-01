from collections.abc import AsyncIterator
from datetime import timedelta

import pytest
from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.balancer_addresses import BalancerAddresses
from paddler_test_cluster.paddler_cluster import paddler_cluster

from paddler_client.balancer_desired_state import BalancerDesiredState

MODEL_LESS_AGENT = AgentSpec(name="model-less-agent", slots=1)
SHORT_BUFFERED_REQUEST_TIMEOUT = timedelta(milliseconds=50)


@pytest.fixture
async def cluster_with_a_model_less_agent() -> AsyncIterator[BalancerAddresses]:
    async with paddler_cluster(BalancerDesiredState(), [MODEL_LESS_AGENT]) as addresses:
        yield addresses


@pytest.fixture
async def cluster_without_agents() -> AsyncIterator[BalancerAddresses]:
    async with paddler_cluster(
        BalancerDesiredState(),
        [],
        buffered_request_timeout=SHORT_BUFFERED_REQUEST_TIMEOUT,
    ) as addresses:
        yield addresses
