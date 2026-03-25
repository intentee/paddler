from __future__ import annotations

import pytest

from paddler_client.client_inference import ClientInference
from paddler_client.inference_message import InferenceMessage, InferenceMessageKind
from paddler_client.types.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.types.continue_from_raw_prompt_params import (
    ContinueFromRawPromptParams,
)
from paddler_client.types.conversation_message import ConversationMessage

pytestmark = pytest.mark.integration


def _assert_not_error(message: InferenceMessage) -> None:
    if message.kind in (
        InferenceMessageKind.TIMEOUT,
        InferenceMessageKind.TOO_MANY_BUFFERED_REQUESTS,
        InferenceMessageKind.SERVER_ERROR,
        InferenceMessageKind.CHAT_TEMPLATE_ERROR,
        InferenceMessageKind.IMAGE_DECODING_FAILED,
    ):
        raise AssertionError(
            f"Unexpected error response: {message.kind}"
            f" ({message.error_message or message.error_code or ''})"
        )


async def test_health(inference_url: str) -> None:
    async with ClientInference(url=inference_url) as client:
        result = await client.get_health()

    assert result == "OK"


async def test_http_continue_from_conversation_history(
    inference_url: str,
    ready_for_inference: None,
) -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Say hello.", role="user"),
        ],
        enable_thinking=False,
        max_tokens=32,
    )

    async with ClientInference(url=inference_url) as client:
        tokens: list[str] = []

        async for message in client.post_continue_from_conversation_history(
            params,
        ):
            _assert_not_error(message)

            if message.kind == InferenceMessageKind.TOKEN:
                assert message.token is not None
                tokens.append(message.token)
            elif message.is_terminal:
                break

    assert len(tokens) > 0


async def test_websocket_continue_from_conversation_history(
    inference_url: str,
    ready_for_inference: None,
) -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Say hi.", role="user"),
        ],
        enable_thinking=False,
        max_tokens=16,
    )

    async with ClientInference(url=inference_url) as client:
        stream = await client.continue_from_conversation_history(params)
        tokens: list[str] = []

        async for message in stream:
            _assert_not_error(message)

            if message.kind == InferenceMessageKind.TOKEN:
                assert message.token is not None
                tokens.append(message.token)
            elif message.is_terminal:
                break

    assert len(tokens) > 0


async def test_websocket_continue_from_raw_prompt(
    inference_url: str,
    ready_for_inference: None,
) -> None:
    params = ContinueFromRawPromptParams(
        max_tokens=16,
        raw_prompt="The meaning of life is",
    )

    async with ClientInference(url=inference_url) as client:
        stream = await client.continue_from_raw_prompt(params)
        tokens: list[str] = []

        async for message in stream:
            _assert_not_error(message)

            if message.kind == InferenceMessageKind.TOKEN:
                assert message.token is not None
                tokens.append(message.token)
            elif message.is_terminal:
                break

    assert len(tokens) > 0
