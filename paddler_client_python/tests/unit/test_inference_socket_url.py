import pytest

from paddler_client.inference_socket_url import inference_socket_url


def test_http_to_ws() -> None:
    result = inference_socket_url("http://localhost:8080")

    assert result == "ws://localhost:8080/api/v1/inference_socket"


def test_https_to_wss() -> None:
    result = inference_socket_url("https://localhost:8080")

    assert result == "wss://localhost:8080/api/v1/inference_socket"


def test_ws_stays_ws() -> None:
    result = inference_socket_url("ws://localhost:8080")

    assert result == "ws://localhost:8080/api/v1/inference_socket"


def test_wss_stays_wss() -> None:
    result = inference_socket_url("wss://localhost:8080")

    assert result == "wss://localhost:8080/api/v1/inference_socket"


def test_replaces_existing_path() -> None:
    result = inference_socket_url("http://example.com:9090/ignored/path")

    assert result == "ws://example.com:9090/api/v1/inference_socket"


def test_preserves_query_parameters() -> None:
    result = inference_socket_url("http://localhost:8080?token=abc")

    assert result == "ws://localhost:8080/api/v1/inference_socket?token=abc"


def test_unsupported_scheme() -> None:
    with pytest.raises(ValueError, match="Unsupported"):
        inference_socket_url("ftp://localhost:8080")
