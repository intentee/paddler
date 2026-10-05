from paddler_client.error import ApiPathWithoutLeadingSlashError


def format_api_url(base_url: str, path: str) -> str:
    if not path.startswith("/"):
        raise ApiPathWithoutLeadingSlashError(path)

    return base_url.rstrip("/") + path
