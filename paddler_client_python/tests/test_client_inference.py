import json

import httpx
import pytest

from paddler_client.client_inference import ClientInference
from paddler_client.error import HttpError
from paddler_client.inference_message import InferenceMessage, InferenceMessageKind
from paddler_client.types.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.types.conversation_message import ConversationMessage
from paddler_client.types.embedding_input_document import EmbeddingInputDocument
from paddler_client.types.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.types.generate_embedding_batch_params import (
    GenerateEmbeddingBatchParams,
)


def _make_ndjson_token_response(
    request_id: str,
    token: str,
) -> str:
    return json.dumps({
        "Response": {
            "request_id": request_id,
            "response": {"GeneratedToken": {"Token": token}},
        }
    })


def _make_ndjson_done_response(request_id: str) -> str:
    return json.dumps({
        "Response": {
            "request_id": request_id,
            "response": {"GeneratedToken": "Done"},
        }
    })


async def test_get_health_returns_text() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "http://test:8080/health"

        return httpx.Response(200, text="OK")

    transport = httpx.MockTransport(handler)
    client = ClientInference(
        url="http://test:8080",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        result = await client.get_health()
        assert result == "OK"
    finally:
        await client.close()


async def test_get_health_raises_on_error() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, text="Error")

    transport = httpx.MockTransport(handler)
    client = ClientInference(
        url="http://test:8080",
        http_client=httpx.AsyncClient(transport=transport),
    )

    try:
        with pytest.raises(HttpError) as exc_info:
            await client.get_health()
        assert exc_info.value.status_code == 500
    finally:
        await client.close()


async def test_post_continue_from_conversation_history() -> None:
    received_requests: list[httpx.Request] = []

    def handler(request: httpx.Request) -> httpx.Response:
        received_requests.append(request)
        ndjson = (
            _make_ndjson_token_response("r1", "hi")
            + "\n"
            + _make_ndjson_done_response("r1")
            + "\n"
        )

        return httpx.Response(200, text=ndjson)

    transport = httpx.MockTransport(handler)
    client = ClientInference(
        url="http://test:8080",
        http_client=httpx.AsyncClient(transport=transport),
    )

    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Hello", role="user"),
        ],
        enable_thinking=False,
        max_tokens=100,
    )

    try:
        messages: list[InferenceMessage] = []

        async for message in client.post_continue_from_conversation_history(
            params,
        ):
            messages.append(message)

        assert len(messages) == 2
        assert messages[0].kind == InferenceMessageKind.TOKEN
        assert messages[0].token == "hi"
        assert messages[1].kind == InferenceMessageKind.DONE

        request_url = str(received_requests[0].url)
        assert "/api/v1/continue_from_conversation_history" in request_url

        body = json.loads(received_requests[0].content)
        assert body["add_generation_prompt"] is True
        assert body["max_tokens"] == 100
        assert body["conversation_history"] == [
            {"content": "Hello", "role": "user"}
        ]
    finally:
        await client.close()


async def test_generate_embedding_batch() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        embedding_response = json.dumps({
            "Response": {
                "request_id": "r1",
                "response": {
                    "Embedding": {
                        "Embedding": {
                            "embedding": [0.1, 0.2],
                            "normalization_method": "None",
                            "pooling_type": "Mean",
                            "source_document_id": "doc-1",
                        }
                    }
                },
            }
        })
        done_response = json.dumps({
            "Response": {
                "request_id": "r1",
                "response": {"Embedding": "Done"},
            }
        })

        return httpx.Response(
            200,
            text=embedding_response + "\n" + done_response + "\n",
        )

    transport = httpx.MockTransport(handler)
    client = ClientInference(
        url="http://test:8080",
        http_client=httpx.AsyncClient(transport=transport),
    )

    params = GenerateEmbeddingBatchParams(
        input_batch=[
            EmbeddingInputDocument(content="test document", id="doc-1"),
        ],
        normalization_method=EmbeddingNormalizationMethod.none(),
    )

    try:
        messages: list[InferenceMessage] = []

        async for message in client.generate_embedding_batch(params):
            messages.append(message)

        assert len(messages) == 2
        assert messages[0].kind == InferenceMessageKind.EMBEDDING
        assert messages[0].embedding_data is not None
        assert messages[0].embedding_data.embedding == [0.1, 0.2]
        assert messages[1].kind == InferenceMessageKind.EMBEDDING_DONE
    finally:
        await client.close()


async def test_context_manager() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text="OK")

    transport = httpx.MockTransport(handler)

    async with ClientInference(
        url="http://test:8080",
        http_client=httpx.AsyncClient(transport=transport),
    ) as client:
        result = await client.get_health()
        assert result == "OK"
