import json
from pathlib import Path


def rewrite_adapter_config(checkpoint_directory: Path, **changes: object) -> None:
    adapter_config_path = checkpoint_directory / "adapter_config.json"
    adapter_config = json.loads(adapter_config_path.read_text(encoding="utf-8"))
    adapter_config_path.write_text(
        json.dumps({**adapter_config, **changes}), encoding="utf-8"
    )
