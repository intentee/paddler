from collections.abc import Iterator
from os import environ

import pytest
from openai import OpenAI


@pytest.fixture
def model() -> str:
    return "qwen3"


@pytest.fixture
def openai_client() -> Iterator[OpenAI]:
    with OpenAI(
        base_url=f"{environ['PADDLER_COMPAT_OPENAI_URL']}/v1", api_key="paddler"
    ) as openai_client:
        yield openai_client
