import pytest

from paddler_client.inference_message import (
    InferenceMessageKind,
    parse_inference_client_message,
)


def test_parse_content_token_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ContentToken": "hello"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.request_id == "req-1"
    assert message.kind == InferenceMessageKind.CONTENT_TOKEN
    assert message.token == "hello"
    assert message.is_token
    assert not message.is_terminal


def test_parse_reasoning_token_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ReasoningToken": "thinking"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.REASONING_TOKEN
    assert message.token == "thinking"
    assert message.is_token
    assert not message.is_terminal


def test_parse_tool_call_token_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ToolCallToken": '{"name":'}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOOL_CALL_TOKEN
    assert message.token == '{"name":'
    assert message.is_token
    assert not message.is_terminal


def test_parse_tool_call_parsed_response_carries_structured_calls() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "ToolCallParsed": [
                        {
                            "id": "call_42",
                            "name": "get_weather",
                            "arguments": {"ValidJson": {"location": "Paris"}},
                        },
                    ],
                },
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOOL_CALL_PARSED
    assert message.parsed_tool_calls is not None
    assert len(message.parsed_tool_calls) == 1
    assert message.parsed_tool_calls[0].id == "call_42"
    assert message.parsed_tool_calls[0].name == "get_weather"
    assert not message.is_token


def test_parse_tool_call_parse_failed_response_carries_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {"ToolCallParseFailed": "syntax error at 12"},
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOOL_CALL_PARSE_FAILED
    assert message.error_message == "syntax error at 12"


def test_parse_tool_call_validation_failed_response_joins_errors() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "ToolCallValidationFailed": [
                        "missing field 'location'",
                        "extra field 'foo'",
                    ],
                },
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOOL_CALL_VALIDATION_FAILED
    assert message.error_message == "missing field 'location'; extra field 'foo'"


def test_parse_tool_call_parsed_with_non_list_payload_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ToolCallParsed": "not a list"}},
        },
    }

    with pytest.raises(TypeError, match="ToolCallParsed payload is not a list"):
        parse_inference_client_message(data)


def test_parse_tool_call_validation_failed_with_non_list_payload_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"ToolCallValidationFailed": "oops"}},
        },
    }

    with pytest.raises(
        TypeError,
        match="ToolCallValidationFailed payload is not a list",
    ):
        parse_inference_client_message(data)


def test_parse_unrecognized_tool_call_format_response_carries_text_and_ffi_error() -> (
    None
):
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "UnrecognizedToolCallFormat": {
                        "text": "<unknown_marker>blah</unknown_marker>",
                        "ffi_error_message": "common_chat_parse failed: no parser",
                    },
                },
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.UNRECOGNIZED_TOOL_CALL_FORMAT
    assert message.raw_tool_call_tokens is not None
    assert message.raw_tool_call_tokens.text == "<unknown_marker>blah</unknown_marker>"
    assert (
        message.raw_tool_call_tokens.ffi_error_message
        == "common_chat_parse failed: no parser"
    )
    assert not message.is_token


def test_parse_unrecognized_tool_call_format_with_non_dict_payload_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {"UnrecognizedToolCallFormat": "raw text only"},
            },
        },
    }

    with pytest.raises(
        TypeError,
        match="UnrecognizedToolCallFormat payload is not a dict",
    ):
        parse_inference_client_message(data)


def test_parse_image_exceeds_batch_size_response_carries_token_counts() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "ImageExceedsBatchSize": {
                        "image_tokens": 368,
                        "n_batch": 100,
                    },
                },
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.IMAGE_EXCEEDS_BATCH_SIZE
    assert message.oversized_image_details is not None
    assert message.oversized_image_details.image_tokens == 368
    assert message.oversized_image_details.n_batch == 100
    assert not message.is_token


def test_parse_image_exceeds_batch_size_with_non_dict_payload_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {"ImageExceedsBatchSize": "scalar payload"},
            },
        },
    }

    with pytest.raises(
        TypeError,
        match="ImageExceedsBatchSize payload is not a dict",
    ):
        parse_inference_client_message(data)


def test_parse_undeterminable_token_response() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": {"UndeterminableToken": "raw"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.UNDETERMINABLE_TOKEN
    assert message.token == "raw"
    assert message.is_token


def test_parse_done_response_carries_summary() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "Done": {
                        "usage": {
                            "prompt_tokens": 4,
                            "cached_prompt_tokens": 0,
                            "input_image_tokens": 0,
                            "input_audio_tokens": 0,
                            "content_tokens": 6,
                            "reasoning_tokens": 1,
                            "tool_call_tokens": 0,
                            "undeterminable_tokens": 0,
                        }
                    }
                }
            },
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.DONE
    assert message.is_done
    assert message.is_terminal
    assert message.summary is not None
    assert message.summary.usage.prompt_tokens == 4
    assert message.summary.usage.content_tokens == 6
    assert message.summary.usage.reasoning_tokens == 1
    assert message.summary.usage.completion_tokens == 7
    assert message.summary.usage.total_tokens == 11


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
            "response": {"GeneratedToken": {"GrammarIncompatibleWithThinking": "err"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_INCOMPATIBLE_WITH_THINKING
    assert message.error_message == "err"
    assert message.is_terminal


def test_parse_grammar_initialization_failed() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {"GrammarInitializationFailed": "null grammar"}
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
                "GeneratedToken": {"GrammarRejectedModelOutput": "token rejected"}
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
            "response": {"GeneratedToken": {"GrammarSyntaxError": "invalid schema"}},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.GRAMMAR_SYNTAX_ERROR
    assert message.error_message == "invalid schema"
    assert message.is_terminal


def test_parse_tool_call_validator_build_failed_response_carries_error() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {
                "GeneratedToken": {
                    "ToolCallValidatorBuildFailed": (
                        'tool "get_weather" parameters are not a valid JSON Schema'
                    ),
                },
            },
        },
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.TOOL_CALL_VALIDATOR_BUILD_FAILED
    assert message.error_message == (
        'tool "get_weather" parameters are not a valid JSON Schema'
    )
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


def test_parse_embedding_no_embeddings_produced() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"Embedding": "NoEmbeddingsProduced"},
        }
    }
    message = parse_inference_client_message(data)

    assert message.kind == InferenceMessageKind.EMBEDDING_NO_EMBEDDINGS_PRODUCED
    assert message.is_terminal


def test_parse_embedding_rejected_due_to_active_token_generation() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"Embedding": "EmbeddingRejectedDueToActiveTokenGeneration"},
        }
    }
    message = parse_inference_client_message(data)

    assert (
        message.kind
        == InferenceMessageKind.EMBEDDING_REJECTED_DUE_TO_ACTIVE_TOKEN_GENERATION
    )
    assert message.is_terminal


def test_parse_json_string() -> None:
    json_str = (
        '{"Response": {"request_id": "req-1", "response": {"GeneratedToken": '
        '{"Done": {"usage": {"prompt_tokens": 0, "cached_prompt_tokens": 0, '
        '"input_image_tokens": 0, "input_audio_tokens": 0, "content_tokens": 0, '
        '"reasoning_tokens": 0, "tool_call_tokens": 0, "undeterminable_tokens": 0}}}}}}'
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


def test_parse_string_generated_token_result_raises() -> None:
    data = {
        "Response": {
            "request_id": "req-1",
            "response": {"GeneratedToken": "Done"},
        }
    }

    with pytest.raises(TypeError, match="Unknown GeneratedTokenResult"):
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
