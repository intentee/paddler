import os

PADDLER_BINARY_ENV = "PADDLER_BINARY"


def paddler_binary_path() -> str:
    return os.environ[PADDLER_BINARY_ENV]
