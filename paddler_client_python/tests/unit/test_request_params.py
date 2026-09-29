from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.embedding_input_document import EmbeddingInputDocument
from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)
from paddler_client.grammar_constraint import (
    GbnfGrammarConstraint,
    JsonSchemaGrammarConstraint,
)


def test_continue_from_conversation_history_params_serialization() -> None:
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


def test_continue_from_raw_prompt_params_serialization() -> None:
    params = ContinueFromRawPromptParams(
        max_tokens=50,
        raw_prompt="Once upon a time",
    )
    dumped = params.model_dump(mode="json")

    assert dumped == {
        "grammar": None,
        "max_tokens": 50,
        "raw_prompt": "Once upon a time",
    }


def test_generate_embedding_batch_params_serialization() -> None:
    params = GenerateEmbeddingBatchParams(
        input_batch=[
            EmbeddingInputDocument(content="hello world", id="doc-1"),
        ],
        normalization_method=EmbeddingNormalizationMethod.l2(),
    )
    dumped = params.model_dump(mode="json")

    assert len(dumped["input_batch"]) == 1
    assert dumped["input_batch"][0]["id"] == "doc-1"
    assert dumped["normalization_method"] == "L2"


def test_raw_prompt_params_with_gbnf_grammar() -> None:
    params = ContinueFromRawPromptParams(
        grammar=GbnfGrammarConstraint(
            grammar='root ::= "yes" | "no"',
            root="root",
        ),
        max_tokens=10,
        raw_prompt="Answer yes or no",
    )
    dumped = params.model_dump(mode="json")

    assert dumped["grammar"]["type"] == "gbnf"
    assert dumped["grammar"]["grammar"] == 'root ::= "yes" | "no"'


def test_conversation_history_params_with_json_schema_grammar() -> None:
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
