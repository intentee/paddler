from typesafe_sdk import SystemOneResponse


def test_system_one_counts_input_tokens_without_output_tokens(
    billing_ticket_answers: SystemOneResponse,
) -> None:
    assert billing_ticket_answers.usage.input_tokens is not None
    assert billing_ticket_answers.usage.input_tokens > 0
    assert billing_ticket_answers.usage.output_tokens == 0
