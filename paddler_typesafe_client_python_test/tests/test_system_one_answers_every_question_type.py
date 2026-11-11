from typesafe_sdk import SystemOneResponse


def test_system_one_answers_every_question_type(
    billing_ticket_answers: SystemOneResponse,
) -> None:
    assert list(billing_ticket_answers.answers) == ["billing", "tone", "urgency"]
    assert 0 <= billing_ticket_answers.nouls["billing"].noul <= 1
    assert billing_ticket_answers.choices["tone"].choice in {"calm", "angry"}
    assert billing_ticket_answers.scores["urgency"].legend == {
        0: "Can wait",
        1: "Needs attention today",
    }
    assert 0 <= billing_ticket_answers.scores["urgency"].score <= 1
