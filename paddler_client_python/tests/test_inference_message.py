import pytest

from paddler_client.inference_message import (
    InferenceMessageKind,
    parse_inference_client_message,
)


def test_parse_token_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"Token": "hello"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.request_id == "req-1"
    assert message.kind == InferenceMessageKind.TOKEN
    assert message.token == "hello"
    assert message.is_token
    assert not message.is_terminal


def test_parse_done_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": "Done"},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.DONE
    assert message.is_done
    assert message.is_terminal


def test_parse_timeout() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": "Timeout",
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TIMEOUT
    assert message.is_terminal


def test_parse_too_many_buffered_requests() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": "TooManyBufferedRequests",
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOO_MANY_BUFFERED_REQUESTS
    assert message.is_terminal


def test_parse_server_error() -> None:
    data = {
        "Error": {
            "request_id": "req-1",
            "error": {"code": 500, "description": "Internal error"},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.SERVER_ERROR
    assert message.error_code == 500
    assert message.error_message == "Internal error"
    assert message.is_terminal


def test_parse_chat_template_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ChatTemplateError": "bad template"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.CHAT_TEMPLATE_ERROR
    assert message.error_message == "bad template"
    assert message.is_terminal


def test_parse_image_decoding_failed() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ImageDecodingFailed": "corrupt image"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.IMAGE_DECODING_FAILED
    assert message.error_message == "corrupt image"
    assert message.is_terminal


def test_parse_multimodal_not_supported() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"MultimodalNotSupported": "no multimodal"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.MULTIMODAL_NOT_SUPPORTED
    assert message.error_message == "no multimodal"
    assert message.is_terminal


def test_parse_embedding_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "Embedding": {
                    "Embedding": {
                        "embedding": [1.0, 2.0, 3.0],
                        "normalization_method": "None",
                        "pooling_type": "Mean",
                        "source_document_id": "doc-1",
                    }
                }
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.EMBEDDING
    assert message.embedding_data is not None
    assert message.embedding_data.embedding == [1.0, 2.0, 3.0]
    assert message.embedding_data.source_document_id == "doc-1"
    assert not message.is_terminal


def test_parse_embedding_done() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"Embedding": "Done"},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.EMBEDDING_DONE
    assert message.is_terminal


def test_parse_embedding_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"Embedding": {"Error": "embedding failed"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.EMBEDDING_ERROR
    assert message.error_message == "embedding failed"
    assert message.is_terminal


def test_parse_json_string() -> None:
    json_str = (
        '{"Response": {"request_id": "req-1", "response": {"GeneratedToken": "Done"}}}'
    )
    message = parse_inference_client_message(json_str)

    assert message.kind == InferenceMessageKind.DONE


def test_parse_unknown_format_raises() -> None:
    with pytest.raises(ValueError, match="Unknown"):
        parse_inference_client_message({"Unknown": {}})


def test_parse_non_dict_raises_type_error() -> None:
    with pytest.raises(TypeError, match="Unknown"):
        parse_inference_client_message(42)  # type: ignore[arg-type]
