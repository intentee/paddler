from typesafe_sdk import SystemOneResponse


def test_system_one_identifies_its_request(
    billing_ticket_answers: SystemOneResponse,
) -> None:
    assert billing_ticket_answers.request_id
