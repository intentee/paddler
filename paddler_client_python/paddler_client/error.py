class PaddlerError(Exception):
    pass


class HttpError(PaddlerError):
    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"HTTP {status_code}: {message}")


class JsonError(PaddlerError):
    def __init__(self, message: str, raw_data: str) -> None:
        self.raw_data = raw_data
        super().__init__(message)


class InvalidSocketPoolSizeError(PaddlerError, ValueError):
    def __init__(self, pool_size: int) -> None:
        self.pool_size = pool_size
        super().__init__(f"pool_size must be >= 1, got {pool_size}")


class ConnectionDroppedError(PaddlerError):
    def __init__(self, request_id: str) -> None:
        self.request_id = request_id
        super().__init__(f"WebSocket connection dropped for request {request_id}")
