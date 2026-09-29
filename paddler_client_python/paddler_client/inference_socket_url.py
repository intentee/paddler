from urllib.parse import urlparse, urlunparse

from paddler_client.error import UnsupportedUrlSchemeError

_SCHEME_MAP: dict[str, str] = {
    "http": "ws",
    "https": "wss",
    "ws": "ws",
    "wss": "wss",
}


def inference_socket_url(url: str) -> str:
    parsed = urlparse(url)
    new_scheme = _SCHEME_MAP.get(parsed.scheme)

    if new_scheme is None:
        raise UnsupportedUrlSchemeError(parsed.scheme)

    return urlunparse(
        (
            new_scheme,
            parsed.netloc,
            "/api/v1/inference_socket",
            "",
            parsed.query,
            "",
        )
    )
