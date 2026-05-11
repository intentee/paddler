from __future__ import annotations

import json
from dataclasses import dataclass
from enum import StrEnum
from typing import Any
from typing import cast

from paddler_client.embedding import Embedding
from paddler_client.oversized_image_details import OversizedImageDetails
from paddler_client.parsed_tool_call import ParsedToolCall
from paddler_client.raw_tool_call_tokens import RawToolCallTokens


class InferenceMessageKind(StrEnum):
    CHAT_TEMPLATE_ERROR = "chat_template_error"
    CONTENT_TOKEN = "content_token"
    DONE = "done"
    EMBEDDING = "embedding"
    EMBEDDING_DONE = "embedding_done"
    EMBEDDING_ERROR = "embedding_error"
    GRAMMAR_INCOMPATIBLE_WITH_THINKING = "grammar_incompatible_with_thinking"
    GRAMMAR_INITIALIZATION_FAILED = "grammar_initialization_failed"
    GRAMMAR_REJECTED_MODEL_OUTPUT = "grammar_rejected_model_output"
    GRAMMAR_SYNTAX_ERROR = "grammar_syntax_error"
    IMAGE_DECODING_FAILED = "image_decoding_failed"
    IMAGE_EXCEEDS_BATCH_SIZE = "image_exceeds_batch_size"
    MULTIMODAL_NOT_SUPPORTED = "multimodal_not_supported"
    REASONING_TOKEN = "reasoning_token"
    SAMPLER_ERROR = "sampler_error"
    SERVER_ERROR = "server_error"
    TIMEOUT = "timeout"
    TOOL_CALL_PARSED = "tool_call_parsed"
    TOOL_CALL_PARSE_FAILED = "tool_call_parse_failed"
    TOOL_CALL_TOKEN = "tool_call_token"
    TOOL_CALL_VALIDATION_FAILED = "tool_call_validation_failed"
    TOOL_CALL_VALIDATOR_BUILD_FAILED = "tool_call_validator_build_failed"
    TOO_MANY_BUFFERED_REQUESTS = "too_many_buffered_requests"
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
            prompt_tokens=int(data.get("prompt_tokens", 0)),
            cached_prompt_tokens=int(data.get("cached_prompt_tokens", 0)),
            input_image_tokens=int(data.get("input_image_tokens", 0)),
            input_audio_tokens=int(data.get("input_audio_tokens", 0)),
            content_tokens=int(data.get("content_tokens", 0)),
            reasoning_tokens=int(data.get("reasoning_tokens", 0)),
            tool_call_tokens=int(data.get("tool_call_tokens", 0)),
            undeterminable_tokens=int(data.get("undeterminable_tokens", 0)),
        )


@dataclass(frozen=True)
class GenerationSummary:
    usage: TokenUsage

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> GenerationSummary:
        return cls(usage=TokenUsage.from_dict(data.get("usage", {})))


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
    generated_by: str | None = None

    @property
    def is_token(self) -> bool:
        return self.kind in _TOKEN_KINDS

    @property
    def is_done(self) -> bool:
        return self.kind == InferenceMessageKind.DONE

    @property
    def is_terminal(self) -> bool:
        return not self.is_token and self.kind != InferenceMessageKind.EMBEDDING


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
            response_envelope.get("generated_by"),
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
    if isinstance(response, str):
        if response == "Timeout":
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.TIMEOUT,
                generated_by=generated_by,
            )

        if response == "TooManyBufferedRequests":
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.TOO_MANY_BUFFERED_REQUESTS,
                generated_by=generated_by,
            )

        msg = f"Unknown response variant: {response}"
        raise ValueError(msg)

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

    msg = f"Unknown response: {response}"
    raise ValueError(msg)


_GENERATED_TOKEN_ERROR_KINDS: dict[str, InferenceMessageKind] = {
    "ChatTemplateError": InferenceMessageKind.CHAT_TEMPLATE_ERROR,
    "GrammarIncompatibleWithThinking": (
        InferenceMessageKind.GRAMMAR_INCOMPATIBLE_WITH_THINKING
    ),
    "GrammarInitializationFailed": InferenceMessageKind.GRAMMAR_INITIALIZATION_FAILED,
    "GrammarRejectedModelOutput": InferenceMessageKind.GRAMMAR_REJECTED_MODEL_OUTPUT,
    "GrammarSyntaxError": InferenceMessageKind.GRAMMAR_SYNTAX_ERROR,
    "ImageDecodingFailed": InferenceMessageKind.IMAGE_DECODING_FAILED,
    "MultimodalNotSupported": InferenceMessageKind.MULTIMODAL_NOT_SUPPORTED,
    "SamplerError": InferenceMessageKind.SAMPLER_ERROR,
    "ToolCallValidatorBuildFailed": InferenceMessageKind.TOOL_CALL_VALIDATOR_BUILD_FAILED,
}


_GENERATED_TOKEN_KINDS: dict[str, InferenceMessageKind] = {
    "ContentToken": InferenceMessageKind.CONTENT_TOKEN,
    "ReasoningToken": InferenceMessageKind.REASONING_TOKEN,
    "ToolCallToken": InferenceMessageKind.TOOL_CALL_TOKEN,
    "UndeterminableToken": InferenceMessageKind.UNDETERMINABLE_TOKEN,
}


def _parse_generated_token_result(
    request_id: str,
    data: str | dict[str, Any],
    generated_by: str | None,
) -> InferenceMessage:
    if not isinstance(data, dict):
        msg = f"Unknown GeneratedTokenResult: {data}"
        raise ValueError(msg)

    if "Done" in data:
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.DONE,
            summary=GenerationSummary.from_dict(data["Done"]),
            generated_by=generated_by,
        )

    if "ToolCallParsed" in data:
        raw_calls = data["ToolCallParsed"]
        if not isinstance(raw_calls, list):
            msg = f"ToolCallParsed payload is not a list: {raw_calls}"
            raise ValueError(msg)
        typed_calls = cast("list[dict[str, Any]]", raw_calls)
        parsed_calls: list[ParsedToolCall] = [
            ParsedToolCall.from_dict(call) for call in typed_calls
        ]
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.TOOL_CALL_PARSED,
            parsed_tool_calls=parsed_calls,
            generated_by=generated_by,
        )

    if "ToolCallParseFailed" in data:
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.TOOL_CALL_PARSE_FAILED,
            error_message=str(data["ToolCallParseFailed"]),
            generated_by=generated_by,
        )

    if "ToolCallValidationFailed" in data:
        errors = data["ToolCallValidationFailed"]
        if not isinstance(errors, list):
            msg = f"ToolCallValidationFailed payload is not a list: {errors}"
            raise ValueError(msg)
        typed_errors = cast("list[object]", errors)
        joined_errors: str = "; ".join(str(error) for error in typed_errors)
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.TOOL_CALL_VALIDATION_FAILED,
            error_message=joined_errors,
            generated_by=generated_by,
        )

    if "UnrecognizedToolCallFormat" in data:
        raw_payload = data["UnrecognizedToolCallFormat"]
        if not isinstance(raw_payload, dict):
            msg = f"UnrecognizedToolCallFormat payload is not a dict: {raw_payload!r}"
            raise ValueError(msg)
        typed_raw = cast("dict[str, Any]", raw_payload)
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.UNRECOGNIZED_TOOL_CALL_FORMAT,
            raw_tool_call_tokens=RawToolCallTokens.from_dict(typed_raw),
            generated_by=generated_by,
        )

    if "ImageExceedsBatchSize" in data:
        details_payload = data["ImageExceedsBatchSize"]
        if not isinstance(details_payload, dict):
            msg = f"ImageExceedsBatchSize payload is not a dict: {details_payload!r}"
            raise ValueError(msg)
        typed_details = cast("dict[str, Any]", details_payload)
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.IMAGE_EXCEEDS_BATCH_SIZE,
            oversized_image_details=OversizedImageDetails.from_dict(typed_details),
            generated_by=generated_by,
        )

    for key, kind in _GENERATED_TOKEN_KINDS.items():
        if key in data:
            return InferenceMessage(
                request_id=request_id,
                kind=kind,
                token=data[key],
                generated_by=generated_by,
            )

    for key, kind in _GENERATED_TOKEN_ERROR_KINDS.items():
        if key in data:
            return InferenceMessage(
                request_id=request_id,
                kind=kind,
                error_message=data[key],
                generated_by=generated_by,
            )

    msg = f"Unknown GeneratedTokenResult: {data}"
    raise ValueError(msg)


def _parse_embedding_result(
    request_id: str,
    data: str | dict[str, Any],
    generated_by: str | None,
) -> InferenceMessage:
    if data == "Done":
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.EMBEDDING_DONE,
            generated_by=generated_by,
        )

    if isinstance(data, dict):
        if "Embedding" in data:
            embedding = Embedding.model_validate(data["Embedding"])

            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.EMBEDDING,
                embedding_data=embedding,
                generated_by=generated_by,
            )

        if "Error" in data:
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.EMBEDDING_ERROR,
                error_message=data["Error"],
                generated_by=generated_by,
            )

    msg = f"Unknown EmbeddingResult: {data}"
    raise ValueError(msg)
