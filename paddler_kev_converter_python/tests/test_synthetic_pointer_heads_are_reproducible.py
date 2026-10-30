from pathlib import Path

from gguf import GGUFReader

from paddler_kev_converter.command_line import main
from tests.tensor_shapes import tensor_shapes


def test_synthetic_pointer_heads_are_reproducible(tmp_path: Path) -> None:
    for written_head in ["first.gguf", "second.gguf"]:
        main(
            [
                "synthetic-pointer-head",
                "--hidden-size",
                "512",
                "--input-normalization",
                "NFC",
                "--output",
                str(tmp_path / written_head),
            ]
        )

    assert (tmp_path / "first.gguf").read_bytes() == (
        tmp_path / "second.gguf"
    ).read_bytes()
    assert tensor_shapes(GGUFReader(tmp_path / "first.gguf"))[
        "pointer_head.key.weight"
    ] == (512, 16)
