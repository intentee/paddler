import asyncio

import pytest

from paddler_client.error import ConnectionDroppedError
from paddler_client.inference_message import InferenceMessage, InferenceMessageKind
from paddler_client.inference_socket_connection import ResponseStream


@pytest.fixture
def token_message() -> InferenceMessage:
    return InferenceMessage(
        request_id="req-1",
        kind=InferenceMessageKind.TOKEN,
        token="hello",
    )


@pytest.fixture
def done_message() -> InferenceMessage:
    return InferenceMessage(
        request_id="req-1",
        kind=InferenceMessageKind.DONE,
    )


async def test_yields_messages_from_queue(
    token_message: InferenceMessage,
    done_message: InferenceMessage,
) -> None:
    queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
    queue.put_nowait(token_message)
    queue.put_nowait(done_message)

    stream = ResponseStream(queue)
    messages: list[InferenceMessage] = []

    async for message in stream:
        messages.append(message)

    assert len(messages) == 2
    assert messages[0].token == "hello"
    assert messages[1].is_done


async def test_stops_after_terminal_message(
    done_message: InferenceMessage,
) -> None:
    queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
    queue.put_nowait(done_message)

    stream = ResponseStream(queue)
    messages: list[InferenceMessage] = []

    async for message in stream:
        messages.append(message)

    assert len(messages) == 1


async def test_raises_exception_from_queue() -> None:
    queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
    queue.put_nowait(ConnectionDroppedError("req-1"))

    stream = ResponseStream(queue)

    with pytest.raises(ConnectionDroppedError):
        async for _message in stream:
            pass


async def test_aiter_returns_self() -> None:
    queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
    stream = ResponseStream(queue)

    assert stream.__aiter__() is stream


async def test_stop_iteration_after_done(
    done_message: InferenceMessage,
) -> None:
    queue: asyncio.Queue[InferenceMessage | Exception] = asyncio.Queue()
    queue.put_nowait(done_message)

    stream = ResponseStream(queue)

    await stream.__anext__()

    with pytest.raises(StopAsyncIteration):
        await stream.__anext__()
