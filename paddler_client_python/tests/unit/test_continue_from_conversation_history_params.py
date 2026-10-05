from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.grammar_constraint import JsonSchemaGrammarConstraint


def test_serializes_the_conversation_history_request() -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Hello!", role="user"),
        ],
        enable_thinking=False,
        max_tokens=100,
    )
    dumped = params.model_dump(mode="json")

    assert dumped["add_generation_prompt"] is True
    assert dumped["conversation_history"] == [{"content": "Hello!", "role": "user"}]
    assert dumped["enable_thinking"] is False
    assert dumped["grammar"] is None
    assert dumped["max_tokens"] == 100
    assert dumped["tools"] == []


def test_serializes_a_json_schema_grammar_under_the_schema_key() -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="hi", role="user"),
        ],
        enable_thinking=False,
        grammar=JsonSchemaGrammarConstraint(
            schema_value='{"type": "object"}',
        ),
        max_tokens=50,
    )
    dumped = params.model_dump(mode="json", by_alias=True)

    assert dumped["grammar"]["type"] == "json_schema"
    assert dumped["grammar"]["schema"] == '{"type": "object"}'
