import os

import pytest
from openai import OpenAI

BASE_URL_ENV = "PADDLER_OPENAI_BASE_URL"
MODEL_ENV = "PADDLER_OPENAI_MODEL"
DEFAULT_MODEL = "qwen3"


@pytest.fixture(scope="session")
def base_url() -> str:
    base_url = os.environ.get(BASE_URL_ENV)

    if not base_url:
        raise RuntimeError(
            f"{BASE_URL_ENV} must point at a running Paddler OpenAI-compatible "
            "endpoint, e.g. http://127.0.0.1:8063/v1 — this suite's sole purpose "
            "is to drive that endpoint with the official OpenAI client."
        )

    return base_url


@pytest.fixture(scope="session")
def model() -> str:
    return os.environ.get(MODEL_ENV, DEFAULT_MODEL)


@pytest.fixture
def openai_client(base_url: str) -> OpenAI:
    return OpenAI(base_url=base_url, api_key="paddler")
