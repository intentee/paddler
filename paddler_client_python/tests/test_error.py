from paddler_client.error import (
    ConnectionDroppedError,
    HttpError,
    JsonError,
    PaddlerError,
    PoolExhaustedError,
    ServerError,
)


def test_http_error_stores_status_code_and_message() -> None:
    error = HttpError(status_code=404, message="Not Found")

    assert error.status_code == 404
    assert error.message == "Not Found"
    assert "404" in str(error)
    assert "Not Found" in str(error)


def test_http_error_inherits_from_paddler_error() -> None:
    error = HttpError(status_code=500, message="Internal")

    assert isinstance(error, PaddlerError)


def test_json_error_stores_raw_data() -> None:
    error = JsonError("parse failed", raw_data="{bad json}")

    assert error.raw_data == "{bad json}"
    assert "parse failed" in str(error)


def test_pool_exhausted_error_message() -> None:
    error = PoolExhaustedError()

    assert "No available WebSocket connections" in str(error)


def test_connection_dropped_error_stores_request_id() -> None:
    error = ConnectionDroppedError(request_id="req-42")

    assert error.request_id == "req-42"
    assert "req-42" in str(error)


def test_server_error_stores_code_and_message() -> None:
    error = ServerError(code=503, message="Overloaded")

    assert error.code == 503
    assert error.message == "Overloaded"
    assert "503" in str(error)
    assert "Overloaded" in str(error)
