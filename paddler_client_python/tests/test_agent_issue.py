import pytest

from paddler_client.agent_issue import AgentIssue


def test_agent_issue_deserialization() -> None:
    issue = AgentIssue.model_validate(
        {"ModelFileDoesNotExist": {"model_path": "/path/to/model"}}
    )

    assert issue.variant == "ModelFileDoesNotExist"
    assert issue.params["model_path"] == "/path/to/model"


def test_agent_issue_serialization() -> None:
    issue = AgentIssue(
        variant="ModelFileDoesNotExist",
        params={"model_path": "/path/to/model"},
    )
    dumped = issue.model_dump(mode="json")

    assert dumped == {"ModelFileDoesNotExist": {"model_path": "/path/to/model"}}


def test_agent_issue_string_variant_deserialization() -> None:
    issue = AgentIssue.model_validate("SimpleIssue")

    assert issue.variant == "SimpleIssue"
    assert issue.params == {}


def test_agent_issue_non_dict_params_deserialization() -> None:
    issue = AgentIssue.model_validate({"SomeIssue": "a string value"})

    assert issue.variant == "SomeIssue"
    assert issue.params == {"value": "a string value"}


def test_agent_issue_invalid_data_raises() -> None:
    with pytest.raises(ValueError, match="Invalid AgentIssue"):
        AgentIssue.model_validate(42)


def test_agent_issue_empty_params_serializes_as_string() -> None:
    issue = AgentIssue(variant="SimpleIssue")
    serialized = issue.model_dump_json()

    assert serialized == '"SimpleIssue"'
