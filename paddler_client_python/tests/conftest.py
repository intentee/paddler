from __future__ import annotations

import asyncio
import os

import pytest

from paddler_client.client_management import ClientManagement
from paddler_client.types.agent_desired_model import AgentDesiredModel
from paddler_client.types.balancer_desired_state import BalancerDesiredState
from paddler_client.types.huggingface_model_reference import (
    HuggingFaceModelReference,
)

POLL_INTERVAL_SECONDS = 0.5
WAIT_FOR_SLOTS_TIMEOUT_SECONDS = 120

TEST_MODEL = AgentDesiredModel.from_huggingface(
    HuggingFaceModelReference(
        filename="Qwen3-0.6B-Q8_0.gguf",
        repo_id="Qwen/Qwen3-0.6B-GGUF",
        revision="main",
    ),
)


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--integration",
        action="store_true",
        default=False,
        help="Run integration tests against a live Paddler instance",
    )


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers",
        "integration: requires a live Paddler instance",
    )


def pytest_collection_modifyitems(
    config: pytest.Config,
    items: list[pytest.Item],
) -> None:
    if config.getoption("--integration"):
        return

    skip = pytest.mark.skip(reason="needs --integration flag")

    for item in items:
        if "integration" in item.keywords:
            item.add_marker(skip)


@pytest.fixture
def inference_url() -> str:
    return os.environ.get("PADDLER_INFERENCE_URL", "http://127.0.0.1:8061")


@pytest.fixture
def management_url() -> str:
    return os.environ.get("PADDLER_MANAGEMENT_URL", "http://127.0.0.1:8060")


async def _wait_for_available_slots(client: ClientManagement) -> None:
    elapsed = 0.0

    while elapsed < WAIT_FOR_SLOTS_TIMEOUT_SECONDS:
        snapshot = await client.get_agents()
        total_slots = sum(agent.slots_total for agent in snapshot.agents)
        total_processing = sum(agent.slots_processing for agent in snapshot.agents)

        if total_slots > 0 and total_processing == 0:
            return

        await asyncio.sleep(POLL_INTERVAL_SECONDS)
        elapsed += POLL_INTERVAL_SECONDS

    msg = f"No idle agent slots within {WAIT_FOR_SLOTS_TIMEOUT_SECONDS}s"
    raise TimeoutError(msg)


@pytest.fixture
async def ready_for_inference(
    management_url: str,
) -> None:
    async with ClientManagement(url=management_url) as client:
        current_state = await client.get_balancer_desired_state()

        if current_state.model != TEST_MODEL:
            desired_state = BalancerDesiredState(
                chat_template_override=current_state.chat_template_override,
                inference_parameters=current_state.inference_parameters,
                model=TEST_MODEL,
                multimodal_projection=current_state.multimodal_projection,
                use_chat_template_override=current_state.use_chat_template_override,
            )

            await client.put_balancer_desired_state(desired_state)

        await _wait_for_available_slots(client)
