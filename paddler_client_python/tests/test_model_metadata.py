from paddler_client.model_metadata import ModelMetadata


def test_model_metadata_deserialization() -> None:
    metadata = ModelMetadata.model_validate(
        {"metadata": {"architecture": "llama", "params": "7B"}}
    )

    assert metadata.metadata["architecture"] == "llama"
    assert metadata.metadata["params"] == "7B"


def test_model_metadata_empty() -> None:
    metadata = ModelMetadata()

    assert metadata.metadata == {}
