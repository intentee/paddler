def format_api_url(base_url: str, path: str) -> str:
    if not path.startswith("/"):
        msg = f"Path must start with '/': {path}"
        raise ValueError(msg)

    return base_url.rstrip("/") + path
