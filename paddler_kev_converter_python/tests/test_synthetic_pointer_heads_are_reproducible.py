from pathlib import Path

from gguf import GGUFReader

from paddler_kev_converter.command_line import main


def test_synthetic_pointer_heads_are_reproducible(tmp_path: Path) -> None:
    for written_head in ["first.gguf", "second.gguf"]:
        main(
            [
                "synthetic-pointer-head",
                "--hidden-size",
                "512",
                "--output",
                str(tmp_path / written_head),
            ]
        )

    assert (tmp_path / "first.gguf").read_bytes() == (
        tmp_path / "second.gguf"
    ).read_bytes()
    assert {
        tensor.name: tuple(int(dimension) for dimension in tensor.shape)
        for tensor in GGUFReader(tmp_path / "first.gguf").tensors
    }["pointer_head.key.weight"] == (512, 16)
