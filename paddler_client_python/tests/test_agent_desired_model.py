import pytest

from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.huggingface_model_reference import (
    HuggingFaceModelReference,
)


def test_agent_desired_model_none_serialization() -> None:
    model = AgentDesiredModel.none()
    serialized = model.model_dump(mode="json")

    assert isinstance(serialized, str)
    assert serialized == "None"


def test_agent_desired_model_none_deserialization() -> None:
    model = AgentDesiredModel.model_validate("None")

    assert model.variant == "None"


def test_agent_desired_model_huggingface_serialization() -> None:
    reference = HuggingFaceModelReference(
        filename="model.gguf",
        repo_id="org/model",
        revision="main",
    )
    model = AgentDesiredModel.from_huggingface(reference)
    dumped = model.model_dump(mode="json")

    assert "HuggingFace" in dumped
    assert dumped["HuggingFace"]["repo_id"] == "org/model"


def test_agent_desired_model_huggingface_deserialization() -> None:
    model = AgentDesiredModel.model_validate(
        {
            "HuggingFace": {
                "filename": "model.gguf",
                "repo_id": "org/model",
                "revision": "main",
            }
        }
    )

    assert model.variant == "HuggingFace"
    assert model.huggingface is not None
    assert model.huggingface.repo_id == "org/model"


def test_agent_desired_model_local_to_agent_serialization() -> None:
    model = AgentDesiredModel.local_to_agent("/path/to/model.gguf")
    dumped = model.model_dump(mode="json")

    assert dumped == {"LocalToAgent": "/path/to/model.gguf"}


def test_agent_desired_model_local_to_agent_deserialization() -> None:
    model = AgentDesiredModel.model_validate({"LocalToAgent": "/path/to/model.gguf"})

    assert model.variant == "LocalToAgent"
    assert model.local_path == "/path/to/model.gguf"


def test_agent_desired_model_invalid_data_raises() -> None:
    with pytest.raises(ValueError, match="Invalid AgentDesiredModel"):
        AgentDesiredModel.model_validate(42)


def test_agent_desired_model_local_to_agent_roundtrip() -> None:
    model = AgentDesiredModel.local_to_agent("/path/to/model")
    dumped = model.model_dump(mode="json")

    assert dumped == {"LocalToAgent": "/path/to/model"}


def test_agent_desired_model_unknown_variant_serialization_raises() -> None:
    model = AgentDesiredModel(variant="Unknown")

    with pytest.raises(ValueError, match="Unknown AgentDesiredModel variant"):
        model.model_dump(mode="json")


def test_agent_desired_model_local_to_agent_missing_path_raises() -> None:
    model = AgentDesiredModel(variant="LocalToAgent", local_path=None)

    with pytest.raises(ValueError, match="local_path is required"):
        model.model_dump(mode="json")
