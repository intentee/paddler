from pathlib import Path

from gguf import GGUFReader

from paddler_kev_converter.command_line import main


def written_malformed_pointer_head(tmp_path: Path, defect: str) -> GGUFReader:
    main(
        [
            "malformed-pointer-head",
            "--defect",
            defect,
            "--hidden-size",
            "64",
            "--input-normalization",
            "NFC",
            "--output",
            str(tmp_path / f"{defect}.gguf"),
        ]
    )

    return GGUFReader(tmp_path / f"{defect}.gguf")
