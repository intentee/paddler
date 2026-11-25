from openai import OpenAI


def test_responses_stream_reaches_completed_and_accumulates_output_text(
    openai_client: OpenAI,
    model: str,
) -> None:
    stream = openai_client.responses.create(
        model=model,
        input="Say hi briefly.",
        max_output_tokens=600,
        stream=True,
    )

    event_types: list[str] = []
    output_text = ""

    for event in stream:
        event_types.append(event.type)

        if event.type == "response.output_text.delta":
            output_text += event.delta

    assert event_types[0] == "response.created"
    assert event_types[-1] == "response.completed"
    assert output_text
