from paddler_client.grammar_constraint import (
    GbnfGrammarConstraint,
    JsonSchemaGrammarConstraint,
)


def test_gbnf_grammar_constraint_serialization() -> None:
    constraint = GbnfGrammarConstraint(
        grammar='root ::= "yes" | "no"',
        root="root",
    )
    dumped = constraint.model_dump(mode="json")

    assert dumped == {
        "type": "gbnf",
        "grammar": 'root ::= "yes" | "no"',
        "root": "root",
    }


def test_json_schema_grammar_constraint_serialization() -> None:
    constraint = JsonSchemaGrammarConstraint(
        schema_value='{"type": "object"}',
    )
    dumped = constraint.model_dump(mode="json", by_alias=True)

    assert dumped == {
        "type": "json_schema",
        "schema": '{"type": "object"}',
    }
