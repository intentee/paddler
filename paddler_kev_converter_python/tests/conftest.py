import json
from os import environ
from pathlib import Path
from shutil import copyfile

import pytest
import torch
from kev.checkpoint import Checkpoint, LoadOptions, read_meta, write_meta

from paddler_kev_converter.command_line import main

KEV_CHECKPOINT = environ["PADDLER_KEV_CHECKPOINT"]


@pytest.fixture(scope="session")
def kev_checkpoint() -> Checkpoint:
    return Checkpoint(KEV_CHECKPOINT)


@pytest.fixture(scope="session")
def lora_kev_run() -> str:
    return KEV_CHECKPOINT


@pytest.fixture(scope="session")
def full_weight_kev_run(
    kev_checkpoint: Checkpoint, tmp_path_factory: pytest.TempPathFactory
) -> str:
    full_weight_directory = tmp_path_factory.mktemp("full_weight")
    _, decision_model = kev_checkpoint.load("cpu", LoadOptions(dtype=torch.bfloat16))
    backbone = decision_model.lm
    backbone.config.tie_word_embeddings = False
    backbone.save_pretrained(full_weight_directory)
    meta = read_meta(kev_checkpoint.path)
    meta.weights = "full"
    meta.weights_dtype = "bf16"
    write_meta(str(full_weight_directory), meta)

    return str(full_weight_directory)


@pytest.fixture(scope="session", params=["lora_kev_run", "full_weight_kev_run"])
def convertible_kev_run(request: pytest.FixtureRequest) -> str:
    run: str = request.getfixturevalue(request.param)

    return run


@pytest.fixture(scope="session")
def converted_backbone_directory(
    convertible_kev_run: str, tmp_path_factory: pytest.TempPathFactory
) -> Path:
    conversion_directory = tmp_path_factory.mktemp("converted")
    main(
        [
            "convert",
            "--checkpoint",
            convertible_kev_run,
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
