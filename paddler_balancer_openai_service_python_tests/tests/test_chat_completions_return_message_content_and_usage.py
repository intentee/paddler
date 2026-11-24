from openai import OpenAI


def test_chat_completions_return_message_content_and_usage(
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
