from __future__ import annotations

import json
from dataclasses import dataclass
from enum import StrEnum
from typing import Any

from paddler_client.embedding import Embedding


class InferenceMessageKind(StrEnum):
    TOKEN = "token"
    DONE = "done"
    CHAT_TEMPLATE_ERROR = "chat_template_error"
    GRAMMAR_INITIALIZATION_FAILED = "grammar_initialization_failed"
    GRAMMAR_SYNTAX_ERROR = "grammar_syntax_error"
    IMAGE_DECODING_FAILED = "image_decoding_failed"
    MULTIMODAL_NOT_SUPPORTED = "multimodal_not_supported"
    EMBEDDING = "embedding"
    EMBEDDING_DONE = "embedding_done"
    EMBEDDING_ERROR = "embedding_error"
    TIMEOUT = "timeout"
    TOO_MANY_BUFFERED_REQUESTS = "too_many_buffered_requests"
    SERVER_ERROR = "server_error"


@dataclass(frozen=True)
class InferenceMessage:
    request_id: str
    kind: InferenceMessageKind
    token: str | None = None
    embedding_data: Embedding | None = None
    error_message: str | None = None
    error_code: int | None = None

    @property
    def is_token(self) -> bool:
        return self.kind == InferenceMessageKind.TOKEN

    @property
    def is_done(self) -> bool:
        return self.kind == InferenceMessageKind.DONE

    @property
    def is_terminal(self) -> bool:
        return self.kind not in (
            InferenceMessageKind.TOKEN,
            InferenceMessageKind.EMBEDDING,
        )


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
) -> InferenceMessage:
    if isinstance(response, str):
        if response == "Timeout":
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.TIMEOUT,
            )

        if response == "TooManyBufferedRequests":
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.TOO_MANY_BUFFERED_REQUESTS,
            )

        msg = f"Unknown response variant: {response}"
        raise ValueError(msg)

    if "GeneratedToken" in response:
        return _parse_generated_token_result(
            request_id,
            response["GeneratedToken"],
        )

    if "Embedding" in response:
        return _parse_embedding_result(
            request_id,
            response["Embedding"],
        )

    msg = f"Unknown response: {response}"
    raise ValueError(msg)


_GENERATED_TOKEN_ERROR_KINDS: dict[str, InferenceMessageKind] = {
    "ChatTemplateError": InferenceMessageKind.CHAT_TEMPLATE_ERROR,
    "GrammarInitializationFailed": InferenceMessageKind.GRAMMAR_INITIALIZATION_FAILED,
    "GrammarSyntaxError": InferenceMessageKind.GRAMMAR_SYNTAX_ERROR,
    "ImageDecodingFailed": InferenceMessageKind.IMAGE_DECODING_FAILED,
    "MultimodalNotSupported": InferenceMessageKind.MULTIMODAL_NOT_SUPPORTED,
}


def _parse_generated_token_result(
    request_id: str,
    data: str | dict[str, Any],
) -> InferenceMessage:
    if data == "Done":
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.DONE,
        )

    if isinstance(data, dict):
        if "Token" in data:
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.TOKEN,
                token=data["Token"],
            )

        for key, kind in _GENERATED_TOKEN_ERROR_KINDS.items():
            if key in data:
                return InferenceMessage(
                    request_id=request_id,
                    kind=kind,
                    error_message=data[key],
                )

    msg = f"Unknown GeneratedTokenResult: {data}"
    raise ValueError(msg)


def _parse_embedding_result(
    request_id: str,
    data: str | dict[str, Any],
) -> InferenceMessage:
    if data == "Done":
        return InferenceMessage(
            request_id=request_id,
            kind=InferenceMessageKind.EMBEDDING_DONE,
        )

    if isinstance(data, dict):
        if "Embedding" in data:
            embedding = Embedding.model_validate(data["Embedding"])

            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.EMBEDDING,
                embedding_data=embedding,
            )

        if "Error" in data:
            return InferenceMessage(
                request_id=request_id,
                kind=InferenceMessageKind.EMBEDDING_ERROR,
                error_message=data["Error"],
            )

    msg = f"Unknown EmbeddingResult: {data}"
    raise ValueError(msg)
