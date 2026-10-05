from paddler_client.huggingface_model_reference import (
    HuggingFaceModelReference,
)


def test_huggingface_model_reference_roundtrip() -> None:
    ref = HuggingFaceModelReference(
        filename="model.gguf",
        repo_id="org/model",
        revision="main",
    )
    dumped = ref.model_dump(mode="json")
    restored = HuggingFaceModelReference.model_validate(dumped)

    assert restored.filename == "model.gguf"
    assert restored.repo_id == "org/model"
    assert restored.revision == "main"
