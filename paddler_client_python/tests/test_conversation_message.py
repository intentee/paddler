from paddler_client.conversation_message import ConversationMessage
from paddler_client.conversation_message_content_part import (
    ImageUrlContentPart,
    TextContentPart,
)
from paddler_client.image_url import ImageUrl


def test_conversation_message_text_content() -> None:
    message = ConversationMessage(content="hello", role="user")
    dumped = message.model_dump(mode="json")

    assert dumped == {"content": "hello", "role": "user"}


def test_conversation_message_parts_content() -> None:
    message = ConversationMessage(
        content=[
            TextContentPart(text="hello"),
            ImageUrlContentPart(
                image_url=ImageUrl(url="http://example.com/img.png"),
            ),
        ],
        role="user",
    )
    dumped = message.model_dump(mode="json")

    assert dumped["content"][0] == {"type": "text", "text": "hello"}
    assert dumped["content"][1] == {
        "type": "image_url",
        "image_url": {"url": "http://example.com/img.png"},
    }


def test_conversation_message_text_deserialization() -> None:
    message = ConversationMessage.model_validate({"content": "hi", "role": "assistant"})

    assert message.content == "hi"
    assert message.role == "assistant"
