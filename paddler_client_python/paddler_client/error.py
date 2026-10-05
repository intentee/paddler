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


class RequestIdInFlightError(PaddlerError, ValueError):
    def __init__(self, request_id: str) -> None:
        self.request_id = request_id
        super().__init__(
            f"Request {request_id} is already in flight on this connection"
        )


class InvalidAgentDesiredModelError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Invalid AgentDesiredModel: {data}")


class AgentDesiredModelLocalPathMissingError(PaddlerError, ValueError):
    def __init__(self) -> None:
        super().__init__("local_path is required for LocalToAgent")


class AgentDesiredModelUrlMissingError(PaddlerError, ValueError):
    def __init__(self) -> None:
        super().__init__("url is required for Url")


class UnknownAgentDesiredModelVariantError(PaddlerError, ValueError):
    def __init__(self, variant: str) -> None:
        self.variant = variant
        super().__init__(f"Unknown AgentDesiredModel variant: {variant}")


class InvalidAgentIssueError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Invalid AgentIssue: {data}")


class InvalidModelDownloadStatusError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Invalid ModelDownloadStatus: {data}")


class InvalidRmsNormPayloadError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Invalid RmsNorm payload: {data}")


class InvalidEmbeddingNormalizationMethodError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Invalid EmbeddingNormalizationMethod: {data}")


class RmsNormEpsilonMissingError(PaddlerError, ValueError):
    def __init__(self) -> None:
        super().__init__("epsilon is required for RmsNorm")


class ApiPathWithoutLeadingSlashError(PaddlerError, ValueError):
    def __init__(self, path: str) -> None:
        self.path = path
        super().__init__(f"Path must start with '/': {path}")


class UnsupportedUrlSchemeError(PaddlerError, ValueError):
    def __init__(self, scheme: str) -> None:
        self.scheme = scheme
        super().__init__(f"Unsupported URL scheme: {scheme}")


class UnknownToolCallArgumentsError(PaddlerError, ValueError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(f"Unknown ToolCallArguments shape: {payload}")


class ToolCallArgumentsNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, arguments: object) -> None:
        self.arguments = arguments
        super().__init__(
            f"arguments field must be a dict (tagged enum), got: {arguments!r}"
        )


class InferenceClientMessageNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Unknown inference client message format: {data}")


class UnknownInferenceClientMessageError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Unknown inference client message format: {data}")


class UnknownResponseVariantError(PaddlerError, ValueError):
    def __init__(self, response: object) -> None:
        self.response = response
        super().__init__(f"Unknown response variant: {response}")


class GeneratedTokenResultNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Unknown GeneratedTokenResult: {data}")


class UnknownGeneratedTokenResultError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Unknown GeneratedTokenResult: {data}")


class UnknownEmbeddingResultError(PaddlerError, ValueError):
    def __init__(self, data: object) -> None:
        self.data = data
        super().__init__(f"Unknown EmbeddingResult: {data}")


class ToolCallParsedPayloadNotAListError(PaddlerError, TypeError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(f"ToolCallParsed payload is not a list: {payload}")


class ToolCallValidationFailedPayloadNotAListError(PaddlerError, TypeError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(f"ToolCallValidationFailed payload is not a list: {payload}")


class UnrecognizedToolCallFormatPayloadNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(
            f"UnrecognizedToolCallFormat payload is not a dict: {payload!r}"
        )


class MediaExceedsMicroBatchPayloadNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(f"MediaExceedsMicroBatch payload is not a dict: {payload!r}")


class PromptExceedsContextSizePayloadNotAnObjectError(PaddlerError, TypeError):
    def __init__(self, payload: object) -> None:
        self.payload = payload
        super().__init__(f"PromptExceedsContextSize payload is not a dict: {payload!r}")


class BatchSizeExceedsContextSizeError(PaddlerError, ValueError):
    def __init__(self, n_batch: int, context_size: int) -> None:
        self.n_batch = n_batch
        self.context_size = context_size
        super().__init__(f"n_batch {n_batch} exceeds context_size {context_size}")


class PenaltiesWithoutWindowError(PaddlerError, ValueError):
    def __init__(self) -> None:
        super().__init__("penalty strengths require a penalty_last_n window")


class PenaltyWindowWithoutPenaltiesError(PaddlerError, ValueError):
    def __init__(self, penalty_last_n: int) -> None:
        self.penalty_last_n = penalty_last_n
        super().__init__(
            f"penalty_last_n {penalty_last_n} requires at least one penalty strength"
        )


class ToolCallParsingWithoutToolsError(PaddlerError, ValueError):
    def __init__(self) -> None:
        super().__init__("parse_tool_calls requires at least one tool")
