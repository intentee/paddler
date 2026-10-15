from pathlib import Path

import numpy as np
from gguf import GGUFReader
from kev.checkpoint import Checkpoint
from kev.model import SPECIAL


def test_converting_writes_the_pointer_head_of_the_checkpoint(
    converted_pointer_head: Path, kev_checkpoint: Checkpoint
) -> None:
    reader = GGUFReader(converted_pointer_head)
    tensors = {tensor.name: tensor.data for tensor in reader.tensors}
    head = kev_checkpoint.meta.head

    assert reader.fields["general.architecture"].contents() == "pointer_head"
    assert reader.fields["pointer_head.temperature"].contents() == np.float32(
        kev_checkpoint.meta.temperature
    )
    assert [
        reader.fields[f"pointer_head.delimiter.{delimiter_name}"].contents()
        for delimiter_name in [
            "state",
            "question",
            "option_start",
            "option_end",
            "decide",
        ]
    ] == SPECIAL
    assert np.array_equal(tensors["pointer_head.key.bias"], head["k.bias"].numpy())
    assert np.array_equal(tensors["pointer_head.key.weight"], head["k.weight"].numpy())
    assert np.array_equal(tensors["pointer_head.query.bias"], head["q.bias"].numpy())
    assert np.array_equal(
        tensors["pointer_head.query.weight"], head["q.weight"].numpy()
    )
