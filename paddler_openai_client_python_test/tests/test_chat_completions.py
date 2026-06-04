from openai import OpenAI


def test_non_streaming_returns_message_content_and_usage(
    openai_client: OpenAI,
    model: str,
) -> None:
    completion = openai_client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": "Say hi briefly."}],
        max_completion_tokens=600,
    )

    assert completion.object == "chat.completion"
    assert completion.choices
    assert completion.choices[0].message.content
    assert completion.usage is not None
    assert completion.usage.total_tokens > 0


def test_streaming_accumulates_content_and_reports_usage(
    openai_client: OpenAI,
    model: str,
) -> None:
    stream = openai_client.chat.completions.create(
        model=model,
        messages=[{"role": "user", "content": "Say hi briefly."}],
        max_completion_tokens=600,
        stream=True,
        stream_options={"include_usage": True},
    )

    content = ""
    finish_reason: str | None = None
    total_tokens = 0

    for chunk in stream:
        assert chunk.object == "chat.completion.chunk"

        if chunk.choices:
            choice = chunk.choices[0]
            if choice.delta.content:
                content += choice.delta.content
            if choice.finish_reason:
                finish_reason = choice.finish_reason

        if chunk.usage is not None:
            total_tokens = chunk.usage.total_tokens

    assert content
    assert finish_reason == "stop"
    assert total_tokens > 0
