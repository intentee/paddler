from pathlib import Path
from shutil import copyfile

import pytest
import torch
from kev.checkpoint import Checkpoint, LoadOptions, read_meta, write_meta
from transformers import AutoModel

from paddler_kev_converter.command_line import main
from tests.kev_checkpoint import KEV_CHECKPOINT

ATTENTION_ONLY_BASE = "Qwen/Qwen3-0.6B-Base"
ATTENTION_ONLY_BASE_REVISION = "da87bfb608c14b7cf20ba1ce41287e8de496c0cd"


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


@pytest.fixture(scope="session")
def attention_only_full_weight_kev_run(
    kev_checkpoint: Checkpoint, tmp_path_factory: pytest.TempPathFactory
) -> str:
    attention_only_directory = tmp_path_factory.mktemp("attention_only_full_weight")
    AutoModel.from_pretrained(
        ATTENTION_ONLY_BASE, revision=ATTENTION_ONLY_BASE_REVISION, dtype=torch.bfloat16
    ).save_pretrained(attention_only_directory)
    meta = read_meta(kev_checkpoint.path)
    meta.base = ATTENTION_ONLY_BASE
    meta.base_revision = ATTENTION_ONLY_BASE_REVISION
    meta.weights = "full"
    meta.weights_dtype = "bf16"
    write_meta(str(attention_only_directory), meta)

    return str(attention_only_directory)


@pytest.fixture(
    scope="session",
    params=["lora_kev_run", "full_weight_kev_run", "attention_only_full_weight_kev_run"],
)
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
