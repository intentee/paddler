from pathlib import Path

from gguf import GGMLQuantizationType, GGUFReader

from paddler_kev_converter.command_line import main


def _written_head(tmp_path: Path, defect: str) -> GGUFReader:
    main(
        [
            "malformed-pointer-head",
            "--defect",
            defect,
            "--hidden-size",
            "64",
            "--output",
            str(tmp_path / f"{defect}.gguf"),
        ]
    )

    return GGUFReader(tmp_path / f"{defect}.gguf")


def test_f16_tensors_are_written_as_f16(tmp_path: Path) -> None:
    assert {
        tensor.tensor_type for tensor in _written_head(tmp_path, "f16-tensors").tensors
    } == {GGMLQuantizationType.F16}


def test_mismatched_projections_differ_in_head_dim(tmp_path: Path) -> None:
    shapes = {
        tensor.name: tuple(int(dimension) for dimension in tensor.shape)
        for tensor in _written_head(tmp_path, "mismatched-projections").tensors
    }

    assert shapes["pointer_head.query.weight"] != shapes["pointer_head.key.weight"]


def test_missing_delimiters_are_left_out(tmp_path: Path) -> None:
    reader = _written_head(tmp_path, "missing-delimiters")

    assert "pointer_head.delimiter.state" not in reader.fields
    assert "pointer_head.temperature" in reader.fields


def test_missing_temperature_is_left_out(tmp_path: Path) -> None:
    reader = _written_head(tmp_path, "missing-temperature")

    assert "pointer_head.temperature" not in reader.fields
    assert "pointer_head.delimiter.state" in reader.fields


def test_plain_text_delimiters_name_no_control_tokens(tmp_path: Path) -> None:
    reader = _written_head(tmp_path, "plain-text-delimiters")

    assert reader.fields["pointer_head.delimiter.state"].contents() == "state"


def test_zero_temperature_is_written(tmp_path: Path) -> None:
    reader = _written_head(tmp_path, "zero-temperature")

    assert reader.fields["pointer_head.temperature"].contents() == 0.0
