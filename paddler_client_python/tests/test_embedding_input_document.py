from paddler_client.embedding_input_document import EmbeddingInputDocument


def test_embedding_input_document_serialization() -> None:
    doc = EmbeddingInputDocument(content="hello world", id="d1")
    dumped = doc.model_dump(mode="json")

    assert dumped == {"content": "hello world", "id": "d1"}
