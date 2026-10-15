from typesafe_sdk import Choice, Noul, Score, TypeSafeClient


def test_system_one_answers_every_question_type(
    typesafe_client: TypeSafeClient,
) -> None:
    result = typesafe_client.system_one(
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

    assert list(result.answers) == ["billing", "tone", "urgency"]
    assert 0 <= result.nouls["billing"].noul <= 1
    assert result.choices["tone"].choice in {"calm", "angry"}
    assert result.scores["urgency"].legend == {
        0: "Can wait",
        1: "Needs attention today",
    }
    assert 0 <= result.scores["urgency"].score <= 1
    assert result.usage.input_tokens is not None
    assert result.usage.input_tokens > 0
    assert result.usage.output_tokens == 0
    assert result.request_id
