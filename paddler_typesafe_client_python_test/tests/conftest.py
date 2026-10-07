from collections.abc import Iterator
from os import environ

import pytest
from typesafe_sdk import TypeSafeClient


@pytest.fixture
def typesafe_client() -> Iterator[TypeSafeClient]:
    with TypeSafeClient(
        api_key="paddler", base_url=environ["PADDLER_COMPAT_TYPESAFE_URL"]
    ) as typesafe_client:
        yield typesafe_client
