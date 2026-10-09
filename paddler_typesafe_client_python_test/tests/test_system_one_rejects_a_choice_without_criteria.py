import pytest
from typesafe_sdk import Choice, TypeSafeClient, TypeSafeUnprocessableEntityError


def test_system_one_rejects_a_choice_without_criteria(
    typesafe_client: TypeSafeClient,
) -> None:
    with pytest.raises(TypeSafeUnprocessableEntityError) as rejection:
        typesafe_client.system_one(
            state="I was charged twice. Please help.",
            questions={"tone": Choice(criteria={})},
        )

    assert rejection.value.request_id
