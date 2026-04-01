from paddler_client.chat_template import ChatTemplate


def test_chat_template_serialization() -> None:
    template = ChatTemplate(content="{% for msg in messages %}...")
    dumped = template.model_dump(mode="json")

    assert dumped == {"content": "{% for msg in messages %}..."}
