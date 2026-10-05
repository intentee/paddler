import pytest

from paddler_client.format_api_url import format_api_url


def test_basic_url() -> None:
    result = format_api_url("http://localhost:8080", "/health")

    assert result == "http://localhost:8080/health"


def test_strips_trailing_slash() -> None:
    result = format_api_url("http://localhost:8080/", "/health")

    assert result == "http://localhost:8080/health"


def test_path_must_start_with_slash() -> None:
    with pytest.raises(ValueError, match="must start with"):
        format_api_url("http://localhost:8080", "health")


def test_api_path() -> None:
    result = format_api_url("http://localhost:8080", "/api/v1/agents")

    assert result == "http://localhost:8080/api/v1/agents"


def test_multiple_trailing_slashes() -> None:
    result = format_api_url("http://localhost:8080///", "/health")

    assert result == "http://localhost:8080/health"
