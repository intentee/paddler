import pytest
from pydantic import ValidationError

from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.error import ToolCallParsingWithoutToolsError
from tests.unit.validation_error_cause import validation_error_cause


def test_rejects_tool_call_parsing_without_tools() -> None:
    with pytest.raises(ValidationError) as validation_error:
        ContinueFromConversationHistoryParams(
            add_generation_prompt=True,
            conversation_history=[
                ConversationMessage(content="Hello!", role="user"),
            ],
            enable_thinking=False,
            max_tokens=100,
            parse_tool_calls=True,
        )

    assert isinstance(
        validation_error_cause(validation_error.value),
        ToolCallParsingWithoutToolsError,
    )
