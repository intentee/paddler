from openai import OpenAI


def test_response_returns_output_text_and_usage(
    openai_client: OpenAI,
    model: str,
) -> None:
    response = openai_client.responses.create(
        model=model,
        input="Say hi briefly.",
        max_output_tokens=600,
    )

    assert response.object == "response"
    assert response.status == "completed"
    assert response.output_text
    assert response.usage is not None
    assert response.usage.total_tokens > 0
