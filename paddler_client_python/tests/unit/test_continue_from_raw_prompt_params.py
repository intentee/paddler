from paddler_client.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.grammar_constraint import GbnfGrammarConstraint


def test_serializes_the_raw_prompt_request() -> None:
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


def test_serializes_a_gbnf_grammar_with_its_type_tag() -> None:
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
