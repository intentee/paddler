from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from typing import Any, cast

from paddler_client.embedding import Embedding
from paddler_client.oversized_embedding_document_details import (
    OversizedEmbeddingDocumentDetails,
)
from paddler_client.oversized_image_details import OversizedImageDetails
from paddler_client.oversized_prompt_details import OversizedPromptDetails
from paddler_client.parsed_tool_call import ParsedToolCall
from paddler_client.raw_tool_call_tokens import RawToolCallTokens


class InferenceMessageKind(StrEnum):
    CHAT_TEMPLATE_ERROR = "chat_template_error"
    CONTENT_TOKEN = "content_token"
    DETOKENIZATION_FAILED = "detokenization_failed"
    DONE = "done"
    EMBEDDING = "embedding"
    EMBEDDING_DOCUMENT_EXCEEDS_BATCH_SIZE = "embedding_document_exceeds_batch_size"
    EMBEDDING_DONE = "embedding_done"
    EMBEDDING_ERROR = "embedding_error"
    EMBEDDING_REJECTED_DUE_TO_ACTIVE_TOKEN_GENERATION = (
        "embedding_rejected_due_to_active_token_generation"
    )
    EMBEDDING_NO_EMBEDDINGS_PRODUCED = "embedding_no_embeddings_produced"
    EMBEDDINGS_DISABLED = "embeddings_disabled"
    GRAMMAR_INCOMPATIBLE_WITH_THINKING = "grammar_incompatible_with_thinking"
    GRAMMAR_INITIALIZATION_FAILED = "grammar_initialization_failed"
    GRAMMAR_REJECTED_MODEL_OUTPUT = "grammar_rejected_model_output"
    GRAMMAR_SYNTAX_ERROR = "grammar_syntax_error"
    IMAGE_DECODING_FAILED = "image_decoding_failed"
    IMAGE_EXCEEDS_BATCH_SIZE = "image_exceeds_batch_size"
    MULTIMODAL_NOT_SUPPORTED = "multimodal_not_supported"
    PROMPT_EXCEEDS_CONTEXT_SIZE = "prompt_exceeds_context_size"
    REASONING_TOKEN = "reasoning_token"
    SAMPLER_ERROR = "sampler_error"
    SERVER_ERROR = "server_error"
    TOKEN_GENERATION_DISABLED = "token_generation_disabled"
    TOOL_CALL_PARSED = "tool_call_parsed"
    TOOL_CALL_PARSE_FAILED = "tool_call_parse_failed"
    TOOL_CALL_TOKEN = "tool_call_token"
    TOOL_CALL_VALIDATION_FAILED = "tool_call_validation_failed"
    TOOL_SCHEMA_INVALID = "tool_schema_invalid"
    UNDETERMINABLE_TOKEN = "undeterminable_token"
    UNRECOGNIZED_TOOL_CALL_FORMAT = "unrecognized_tool_call_format"


_TOKEN_KINDS: frozenset[InferenceMessageKind] = frozenset(
    {
        InferenceMessageKind.CONTENT_TOKEN,
        InferenceMessageKind.REASONING_TOKEN,
        InferenceMessageKind.TOOL_CALL_TOKEN,
        InferenceMessageKind.UNDETERMINABLE_TOKEN,
    },
)

_NON_TERMINAL_KINDS: frozenset[InferenceMessageKind] = _TOKEN_KINDS | frozenset(
    {
        InferenceMessageKind.EMBEDDING,
        InferenceMessageKind.EMBEDDING_DOCUMENT_EXCEEDS_BATCH_SIZE,
        InferenceMessageKind.TOOL_CALL_PARSED,
        InferenceMessageKind.TOOL_CALL_PARSE_FAILED,
        InferenceMessageKind.TOOL_CALL_VALIDATION_FAILED,
        InferenceMessageKind.UNRECOGNIZED_TOOL_CALL_FORMAT,
    },
)


@dataclass(frozen=True)
class TokenUsage:
    prompt_tokens: int = 0
    cached_prompt_tokens: int = 0
    input_image_tokens: int = 0
    input_audio_tokens: int = 0
    content_tokens: int = 0
    reasoning_tokens: int = 0
    tool_call_tokens: int = 0
    undeterminable_tokens: int = 0

    @property
    def completion_tokens(self) -> int:
        return (
            self.content_tokens
            + self.reasoning_tokens
            + self.tool_call_tokens
            + self.undeterminable_tokens
        )

    @property
    def total_tokens(self) -> int:
        return self.prompt_tokens + self.completion_tokens

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> TokenUsage:
        return cls(
            prompt_tokens=int(data["prompt_tokens"]),
            cached_prompt_tokens=int(data["cached_prompt_tokens"]),
            input_image_tokens=int(data["input_image_tokens"]),
            input_audio_tokens=int(data["input_audio_tokens"]),
            content_tokens=int(data["content_tokens"]),
            reasoning_tokens=int(data["reasoning_tokens"]),
            tool_call_tokens=int(data["tool_call_tokens"]),
            undeterminable_tokens=int(data["undeterminable_tokens"]),
        )


@dataclass(frozen=True)
class GenerationSummary:
    usage: TokenUsage

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> GenerationSummary:
        return cls(usage=TokenUsage.from_dict(data["usage"]))


@dataclass(frozen=True)
class InferenceMessage:
    request_id: str
    kind: InferenceMessageKind
    token: str | None = None
    embedding_data: Embedding | None = None
    error_message: str | None = None
    error_code: int | None = None
    summary: GenerationSummary | None = None
    parsed_tool_calls: list[ParsedToolCall] | None = None
    raw_tool_call_tokens: RawToolCallTokens | None = None
    oversized_image_details: OversizedImageDetails | None = None
    oversized_prompt_details: OversizedPromptDetails | None = None
    oversized_embedding_document_details: OversizedEmbeddingDocumentDetails | None = (
        None
    )
    generated_by: str | None = None

    @property
    def is_token(self) -> bool:
        return self.kind in _TOKEN_KINDS

    @property
    def is_done(self) -> bool:
        return self.kind == InferenceMessageKind.DONE

    @property
    def is_terminal(self) -> bool:
        return self.kind not in _NON_TERMINAL_KINDS


def parse_inference_client_message(
    data: str | dict[str, Any],
) -> InferenceMessage:
    if isinstance(data, str):
        data = json.loads(data)

    if not isinstance(data, dict):
        msg = f"Unknown inference client message format: {data}"
        raise TypeError(msg)

    if "Error" in data:
        return _parse_error_envelope(data["Error"])

    if "Response" in data:
        response_envelope = data["Response"]

        return _parse_response(
            response_envelope["request_id"],
            response_envelope["response"],
            response_envelope["generated_by"],
        )

    msg = f"Unknown inference client message format: {data}"
    raise ValueError(msg)


def _parse_error_envelope(
    error_envelope: dict[str, Any],
) -> InferenceMessage:
    error = error_envelope["error"]

    return InferenceMessage(
        request_id=error_envelope["request_id"],
        kind=InferenceMessageKind.SERVER_ERROR,
        error_code=error["code"],
        error_message=error["description"],
    )


def _parse_response(
    request_id: str,
    response: str | dict[str, Any],
    generated_by: str | None,
) -> InferenceMessage:
    if isinstance(response, dict):
        if "GeneratedToken" in response:
            return _parse_generated_token_result(
                request_id,
                response["GeneratedToken"],
                generated_by,
            )

        if "Embedding" in response:
            return _parse_embedding_result(
                request_id,
                response["Embedding"],
                generated_by,
            )

    msg = f"Unknown response variant: {response}"
    raise ValueError(msg)


_GENERATED_TOKEN_ERROR_KINDS: dict[str, InferenceMessageKind] = {
    "ChatTemplateError": InferenceMessageKind.CHAT_TEMPLATE_ERROR,
    "DetokenizationFailed": InferenceMessageKind.DETOKENIZATION_FAILED,
    "GrammarIncompatibleWithThinking": (
        InferenceMessageKind.GRAMMAR_INCOMPATIBLE_WITH_THINKING
    ),
    "GrammarInitializationFailed": InferenceMessageKind.GRAMMAR_INITIALIZATION_FAILED,
    "GrammarRejectedModelOutput": InferenceMessageKind.GRAMMAR_REJECTED_MODEL_OUTPUT,
    "GrammarSyntaxError": InferenceMessageKind.GRAMMAR_SYNTAX_ERROR,
    "ImageDecodingFailed": InferenceMessageKind.IMAGE_DECODING_FAILED,
    "MultimodalNotSupported": InferenceMessageKind.MULTIMODAL_NOT_SUPPORTED,
    "SamplerError": InferenceMessageKind.SAMPLER_ERROR,
    "TokenGenerationDisabled": InferenceMessageKind.TOKEN_GENERATION_DISABLED,
    "ToolSchemaInvalid": InferenceMessageKind.TOOL_SCHEMA_INVALID,
}


_GENERATED_TOKEN_KINDS: dict[str, InferenceMessageKind] = {
    "ContentToken": InferenceMessageKind.CONTENT_TOKEN,
    "ReasoningToken": InferenceMessageKind.REASONING_TOKEN,
    "ToolCallToken": InferenceMessageKind.TOOL_CALL_TOKEN,
    "UndeterminableToken": InferenceMessageKind.UNDETERMINABLE_TOKEN,
}


def _build_done_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.DONE,
        summary=GenerationSummary.from_dict(payload),
        generated_by=generated_by,
    )


def _build_tool_call_parsed_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(payload, list):
        msg = f"ToolCallParsed payload is not a list: {payload}"
        raise TypeError(msg)
    typed_calls = cast("list[dict[str, Any]]", payload)
    parsed_calls: list[ParsedToolCall] = [
        ParsedToolCall.from_dict(call) for call in typed_calls
    ]
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.TOOL_CALL_PARSED,
        parsed_tool_calls=parsed_calls,
        generated_by=generated_by,
    )


def _build_tool_call_parse_failed_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.TOOL_CALL_PARSE_FAILED,
        error_message=str(payload),
        generated_by=generated_by,
    )


def _build_tool_call_validation_failed_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(payload, list):
        msg = f"ToolCallValidationFailed payload is not a list: {payload}"
        raise TypeError(msg)
    typed_errors = cast("list[object]", payload)
    joined_errors: str = "; ".join(str(error) for error in typed_errors)
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.TOOL_CALL_VALIDATION_FAILED,
        error_message=joined_errors,
        generated_by=generated_by,
    )


def _build_unrecognized_tool_call_format_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(payload, dict):
        msg = f"UnrecognizedToolCallFormat payload is not a dict: {payload!r}"
        raise TypeError(msg)
    typed_raw = cast("dict[str, Any]", payload)
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.UNRECOGNIZED_TOOL_CALL_FORMAT,
        raw_tool_call_tokens=RawToolCallTokens.from_dict(typed_raw),
        generated_by=generated_by,
    )


def _build_image_exceeds_batch_size_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(payload, dict):
        msg = f"ImageExceedsBatchSize payload is not a dict: {payload!r}"
        raise TypeError(msg)
    typed_details = cast("dict[str, Any]", payload)
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.IMAGE_EXCEEDS_BATCH_SIZE,
        oversized_image_details=OversizedImageDetails.from_dict(typed_details),
        generated_by=generated_by,
    )


def _build_prompt_exceeds_context_size_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(payload, dict):
        msg = f"PromptExceedsContextSize payload is not a dict: {payload!r}"
        raise TypeError(msg)
    typed_details = cast("dict[str, Any]", payload)
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.PROMPT_EXCEEDS_CONTEXT_SIZE,
        oversized_prompt_details=OversizedPromptDetails.from_dict(typed_details),
        generated_by=generated_by,
    )


def _build_token_kind_message(
    request_id: str,
    kind: InferenceMessageKind,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=kind,
        token=payload,
        generated_by=generated_by,
    )


def _build_error_kind_message(
    request_id: str,
    kind: InferenceMessageKind,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=kind,
        error_message=payload,
        generated_by=generated_by,
    )


_StructuredHandler = Callable[[str, Any, str | None], InferenceMessage]

_STRUCTURED_HANDLERS: dict[str, _StructuredHandler] = {
    "Done": _build_done_message,
    "ToolCallParsed": _build_tool_call_parsed_message,
    "ToolCallParseFailed": _build_tool_call_parse_failed_message,
    "ToolCallValidationFailed": _build_tool_call_validation_failed_message,
    "UnrecognizedToolCallFormat": _build_unrecognized_tool_call_format_message,
    "ImageExceedsBatchSize": _build_image_exceeds_batch_size_message,
    "PromptExceedsContextSize": _build_prompt_exceeds_context_size_message,
}


def _parse_generated_token_result(
    request_id: str,
    data: str | dict[str, Any],
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(data, dict):
        msg = f"Unknown GeneratedTokenResult: {data}"
        raise TypeError(msg)
    for structured_key, handler in _STRUCTURED_HANDLERS.items():
        if structured_key in data:
            return handler(request_id, data[structured_key], generated_by)
    for token_key, token_kind in _GENERATED_TOKEN_KINDS.items():
        if token_key in data:
            return _build_token_kind_message(
                request_id,
                token_kind,
                data[token_key],
                generated_by,
            )
    for error_key, error_kind in _GENERATED_TOKEN_ERROR_KINDS.items():
        if error_key in data:
            return _build_error_kind_message(
                request_id,
                error_kind,
                data[error_key],
                generated_by,
            )
    msg = f"Unknown GeneratedTokenResult: {data}"
    raise ValueError(msg)


_EMBEDDING_UNIT_KINDS: dict[str, InferenceMessageKind] = {
    "Done": InferenceMessageKind.EMBEDDING_DONE,
    "EmbeddingRejectedDueToActiveTokenGeneration": (
        InferenceMessageKind.EMBEDDING_REJECTED_DUE_TO_ACTIVE_TOKEN_GENERATION
    ),
    "EmbeddingsDisabled": InferenceMessageKind.EMBEDDINGS_DISABLED,
    "NoEmbeddingsProduced": InferenceMessageKind.EMBEDDING_NO_EMBEDDINGS_PRODUCED,
}


def _build_embedding_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.EMBEDDING,
        embedding_data=Embedding.model_validate(payload),
        generated_by=generated_by,
    )


def _build_embedding_error_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.EMBEDDING_ERROR,
        error_message=payload,
        generated_by=generated_by,
    )


def _build_embedding_document_exceeds_batch_size_message(
    request_id: str,
    payload: Any,
    generated_by: str | None,
) -> InferenceMessage:
    return InferenceMessage(
        request_id=request_id,
        kind=InferenceMessageKind.EMBEDDING_DOCUMENT_EXCEEDS_BATCH_SIZE,
        oversized_embedding_document_details=(
            OversizedEmbeddingDocumentDetails.from_dict(payload)
        ),
        generated_by=generated_by,
    )


_EMBEDDING_STRUCTURED_HANDLERS: dict[str, _StructuredHandler] = {
    "DocumentExceedsBatchSize": _build_embedding_document_exceeds_batch_size_message,
    "Embedding": _build_embedding_message,
    "Error": _build_embedding_error_message,
}


def _parse_embedding_result(
    request_id: str,
    data: str | dict[str, Any],
    generated_by: str | None,
) -> InferenceMessage:
    if isinstance(data, str) and data in _EMBEDDING_UNIT_KINDS:
        return InferenceMessage(
            request_id=request_id,
            kind=_EMBEDDING_UNIT_KINDS[data],
            generated_by=generated_by,
        )

    if isinstance(data, dict):
        for structured_key, handler in _EMBEDDING_STRUCTURED_HANDLERS.items():
            if structured_key in data:
                return handler(request_id, data[structured_key], generated_by)

    msg = f"Unknown EmbeddingResult: {data}"
    raise ValueError(msg)
