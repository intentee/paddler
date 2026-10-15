from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from gguf import GGUFWriter
from numpy.typing import NDArray

type ProjectionArray = NDArray[np.float16] | NDArray[np.float32]

POINTER_HEAD_ARCHITECTURE = "pointer_head"
DELIMITER_NAMES = ["state", "question", "option_start", "option_end", "decide"]


@dataclass(frozen=True)
class PointerHead:
    delimiters: list[str]
    key_bias: ProjectionArray
    key_weight: ProjectionArray
    query_bias: ProjectionArray
    query_weight: ProjectionArray
    temperature: float

    def add_delimiters(self, writer: GGUFWriter) -> None:
        for delimiter_name, delimiter_token in zip(
            DELIMITER_NAMES, self.delimiters, strict=True
        ):
            writer.add_string(
                f"{POINTER_HEAD_ARCHITECTURE}.delimiter.{delimiter_name}",
                delimiter_token,
            )

    def add_temperature(self, writer: GGUFWriter) -> None:
        writer.add_float32(f"{POINTER_HEAD_ARCHITECTURE}.temperature", self.temperature)

    def add_tensors(self, writer: GGUFWriter) -> None:
        writer.add_tensor(f"{POINTER_HEAD_ARCHITECTURE}.key.bias", self.key_bias)
        writer.add_tensor(f"{POINTER_HEAD_ARCHITECTURE}.key.weight", self.key_weight)
        writer.add_tensor(f"{POINTER_HEAD_ARCHITECTURE}.query.bias", self.query_bias)
        writer.add_tensor(
            f"{POINTER_HEAD_ARCHITECTURE}.query.weight", self.query_weight
        )

    def write_gguf(self, path: Path) -> None:
        self.write_gguf_sections(
            path, [self.add_temperature, self.add_delimiters, self.add_tensors]
        )

    def write_gguf_sections(
        self, path: Path, sections: list[Callable[[GGUFWriter], None]]
    ) -> None:
        writer = GGUFWriter(path, POINTER_HEAD_ARCHITECTURE)

        for add_section in sections:
            add_section(writer)

        writer.write_header_to_file()
        writer.write_kv_data_to_file()
        writer.write_tensors_to_file()
        writer.close()
