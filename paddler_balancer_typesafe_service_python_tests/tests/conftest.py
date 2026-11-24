from collections.abc import Iterator
from os import environ

import pytest
from typesafe_sdk import Choice, Noul, Score, SystemOneResponse, TypeSafeClient


@pytest.fixture
def typesafe_client() -> Iterator[TypeSafeClient]:
    with TypeSafeClient(
        api_key="paddler", base_url=environ["PADDLER_COMPAT_TYPESAFE_URL"]
    ) as typesafe_client:
        yield typesafe_client


@pytest.fixture
def billing_ticket_answers(typesafe_client: TypeSafeClient) -> SystemOneResponse:
    return typesafe_client.system_one(
        state={"customer": "Ada", "message": "I was charged twice. Please help."},
        questions={
            "billing": Noul(instructions="Is this about billing?"),
            "tone": Choice(
                instructions="What is the tone?",
                criteria={"calm": None, "angry": "Shouting or insults"},
            ),
            "urgency": Score(criteria=["Can wait", "Needs attention today"]),
        },
    )
