from collections.abc import Callable
from dataclasses import replace
from enum import StrEnum
from pathlib import Path

import numpy as np

from paddler_kev_converter.pointer_head import PointerHead
from paddler_kev_converter.synthetic_pointer_head import synthetic_pointer_head

OVERSIZED_HEAD_DIM = 2**16
PLAIN_TEXT_DELIMITERS = ["state", "question", "option", "end", "decide"]


class PointerHeadDefect(StrEnum):
    F16_TENSORS = "f16-tensors"
    MISMATCHED_PROJECTIONS = "mismatched-projections"
    MISSING_DELIMITERS = "missing-delimiters"
    MISSING_TEMPERATURE = "missing-temperature"
    OVERSIZED_HEAD_DIM = "oversized-head-dim"
    PLAIN_TEXT_DELIMITERS = "plain-text-delimiters"
    ZERO_TEMPERATURE = "zero-temperature"


def _write_with_f16_tensors(well_formed_head: PointerHead, path: Path) -> None:
    replace(
        well_formed_head,
        key_bias=well_formed_head.key_bias.astype(np.float16),
        key_weight=well_formed_head.key_weight.astype(np.float16),
        query_bias=well_formed_head.query_bias.astype(np.float16),
        query_weight=well_formed_head.query_weight.astype(np.float16),
    ).write_gguf(path)


def _write_with_mismatched_projections(
    well_formed_head: PointerHead, path: Path
) -> None:
    replace(
        well_formed_head,
        query_bias=well_formed_head.query_bias[1:],
        query_weight=well_formed_head.query_weight[1:],
    ).write_gguf(path)


def _write_without_delimiters(well_formed_head: PointerHead, path: Path) -> None:
    well_formed_head.write_gguf_sections(
        path, [well_formed_head.add_temperature, well_formed_head.add_tensors]
    )


def _write_without_temperature(well_formed_head: PointerHead, path: Path) -> None:
    well_formed_head.write_gguf_sections(
        path, [well_formed_head.add_delimiters, well_formed_head.add_tensors]
    )


def _write_with_oversized_head_dim(well_formed_head: PointerHead, path: Path) -> None:
    hidden_size = well_formed_head.key_weight.shape[1]

    replace(
        well_formed_head,
        key_bias=np.zeros(OVERSIZED_HEAD_DIM, dtype=np.float32),
        key_weight=np.zeros((OVERSIZED_HEAD_DIM, hidden_size), dtype=np.float32),
        query_bias=np.zeros(OVERSIZED_HEAD_DIM, dtype=np.float32),
        query_weight=np.zeros((OVERSIZED_HEAD_DIM, hidden_size), dtype=np.float32),
    ).write_gguf(path)


def _write_with_plain_text_delimiters(
    well_formed_head: PointerHead, path: Path
) -> None:
    replace(well_formed_head, delimiters=PLAIN_TEXT_DELIMITERS).write_gguf(path)


def _write_with_zero_temperature(well_formed_head: PointerHead, path: Path) -> None:
    replace(well_formed_head, temperature=0.0).write_gguf(path)


DEFECT_WRITERS: dict[PointerHeadDefect, Callable[[PointerHead, Path], None]] = {
    PointerHeadDefect.F16_TENSORS: _write_with_f16_tensors,
    PointerHeadDefect.MISMATCHED_PROJECTIONS: _write_with_mismatched_projections,
    PointerHeadDefect.MISSING_DELIMITERS: _write_without_delimiters,
    PointerHeadDefect.MISSING_TEMPERATURE: _write_without_temperature,
    PointerHeadDefect.OVERSIZED_HEAD_DIM: _write_with_oversized_head_dim,
    PointerHeadDefect.PLAIN_TEXT_DELIMITERS: _write_with_plain_text_delimiters,
    PointerHeadDefect.ZERO_TEMPERATURE: _write_with_zero_temperature,
}


def write_malformed_pointer_head(
    defect: PointerHeadDefect, hidden_size: int, path: Path
) -> None:
    DEFECT_WRITERS[defect](synthetic_pointer_head(hidden_size), path)
