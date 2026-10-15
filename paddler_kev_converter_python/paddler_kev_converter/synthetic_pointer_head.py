from math import sqrt

import numpy as np
from kev.model import SPECIAL
from numpy.typing import NDArray

from paddler_kev_converter.pointer_head import PointerHead

SYNTHETIC_HEAD_DIM = 16
SYNTHETIC_SEED = 20261006
SYNTHETIC_TEMPERATURE = 1.5


def synthetic_pointer_head(hidden_size: int) -> PointerHead:
    generator = np.random.default_rng(SYNTHETIC_SEED)
    weight_scale = 1 / sqrt(hidden_size)

    def projection_weight() -> NDArray[np.float32]:
        return (
            generator.standard_normal((SYNTHETIC_HEAD_DIM, hidden_size)) * weight_scale
        ).astype(np.float32)

    def projection_bias() -> NDArray[np.float32]:
        return generator.standard_normal(SYNTHETIC_HEAD_DIM).astype(np.float32)

    return PointerHead(
        delimiters=SPECIAL,
        key_bias=projection_bias(),
        key_weight=projection_weight(),
        query_bias=projection_bias(),
        query_weight=projection_weight(),
        temperature=SYNTHETIC_TEMPERATURE,
    )
