import pytest
from typesafe_sdk import Choice, TypeSafeClient, TypeSafeUnprocessableEntityError

MOST_CRITERIA_PADDLER_ACCEPTS_IN_A_CHOICE = 255


def test_system_one_rejects_a_choice_with_more_criteria_than_paddler_accepts(
    typesafe_client: TypeSafeClient,
) -> None:
    with pytest.raises(TypeSafeUnprocessableEntityError) as rejection:
        typesafe_client.system_one(
            state="I was charged twice. Please help.",
            questions={
                "tone": Choice(
                    criteria={
                        f"option_{option_index}": None
                        for option_index in range(
                            MOST_CRITERIA_PADDLER_ACCEPTS_IN_A_CHOICE + 1
                        )
                    }
                )
            },
        )

    assert rejection.value.request_id
