def format_api_url(base_url: str, path: str) -> str:
    if not path.startswith("/"):
        raise ValueError(f"Path must start with '/': {path}")

    return base_url.rstrip("/") + path
