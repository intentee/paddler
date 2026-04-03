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


def test_parse_grammar_incompatible_with_thinking() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "GrammarIncompatibleWithThinking": "cannot use grammar with thinking"
                }
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_INCOMPATIBLE_WITH_THINKING
    assert message.error_message == "cannot use grammar with thinking"
    assert message.is_terminal


def test_parse_grammar_initialization_failed() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "GrammarInitializationFailed": "null grammar"
                }
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_INITIALIZATION_FAILED
    assert message.error_message == "null grammar"
    assert message.is_terminal


def test_parse_grammar_rejected_model_output() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "GrammarRejectedModelOutput": "token rejected"
                }
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_REJECTED_MODEL_OUTPUT
    assert message.error_message == "token rejected"
    assert message.is_terminal


def test_parse_grammar_syntax_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {"GrammarSyntaxError": "invalid schema"}
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_SYNTAX_ERROR
    assert message.error_message == "invalid schema"
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


def test_parse_sampler_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"SamplerError": "no candidates"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.SAMPLER_ERROR
    assert message.error_message == "no candidates"
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


def test_parse_unknown_response_variant_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": "SomethingUnexpected",
        }
    }

    with pytest.raises(ValueError, match="Unknown response variant"):
        parse_inference_client_message(data)


def test_parse_unknown_response_dict_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"UnknownKey": "value"},
        }
    }

    with pytest.raises(ValueError, match="Unknown response"):
        parse_inference_client_message(data)


def test_parse_unknown_generated_token_result_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"UnknownVariant": "data"}},
        }
    }

    with pytest.raises(ValueError, match="Unknown GeneratedTokenResult"):
        parse_inference_client_message(data)


def test_parse_unknown_embedding_result_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"Embedding": {"UnknownVariant": "data"}},
        }
    }

    with pytest.raises(ValueError, match="Unknown EmbeddingResult"):
        parse_inference_client_message(data)
