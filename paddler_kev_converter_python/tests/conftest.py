import json
from os import environ
from pathlib import Path
from shutil import copyfile

import pytest
from kev.checkpoint import Checkpoint

from paddler_kev_converter.command_line import main

KEV_CHECKPOINT = environ["PADDLER_KEV_CHECKPOINT"]


@pytest.fixture(scope="session")
def kev_checkpoint() -> Checkpoint:
    return Checkpoint(KEV_CHECKPOINT)


@pytest.fixture(scope="session")
def converted_backbone_directory(tmp_path_factory: pytest.TempPathFactory) -> Path:
    conversion_directory = tmp_path_factory.mktemp("converted")
    main(
        [
            "convert",
            "--checkpoint",
            KEV_CHECKPOINT,
            "--backbone-directory",
            str(conversion_directory / "backbone"),
            "--pointer-head",
            str(conversion_directory / "pointer_head.gguf"),
        ]
    )

    return conversion_directory / "backbone"


@pytest.fixture(scope="session")
def converted_pointer_head(converted_backbone_directory: Path) -> Path:
    return converted_backbone_directory.parent / "pointer_head.gguf"


@pytest.fixture
def kev_checkpoint_copy(kev_checkpoint: Checkpoint, tmp_path: Path) -> Path:
    for checkpoint_file in Path(kev_checkpoint.path).iterdir():
        copyfile(checkpoint_file, tmp_path / checkpoint_file.name)

    return tmp_path


def rewrite_adapter_config(checkpoint_directory: Path, **changes: object) -> None:
    adapter_config_path = checkpoint_directory / "adapter_config.json"
    adapter_config = json.loads(adapter_config_path.read_text(encoding="utf-8"))
    adapter_config_path.write_text(
        json.dumps({**adapter_config, **changes}), encoding="utf-8"
    )
