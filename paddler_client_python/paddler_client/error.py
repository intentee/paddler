class PaddlerError(Exception):
    pass


class HttpError(PaddlerError):
    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"HTTP {status_code}: {message}")


class WebSocketError(PaddlerError):
    pass


class JsonError(PaddlerError):
    def __init__(self, message: str, raw_data: str) -> None:
        self.raw_data = raw_data
        super().__init__(message)


class PoolExhaustedError(PaddlerError):
    def __init__(self) -> None:
        super().__init__(
            "No available WebSocket connections in the pool"
        )


class ConnectionDroppedError(PaddlerError):
    def __init__(self, request_id: str) -> None:
        self.request_id = request_id
        super().__init__(
            f"WebSocket connection dropped for request {request_id}"
        )


class ServerError(PaddlerError):
    def __init__(self, code: int, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"Server error (code={code}): {message}")
