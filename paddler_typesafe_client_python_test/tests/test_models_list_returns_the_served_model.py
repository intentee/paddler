from os.path import normpath
from pathlib import Path

from typesafe_sdk import TypeSafeClient

KEV_0_8B_MODEL_PATH = normpath(
    Path(__file__).parents[2] / "target" / "kev" / "model.gguf"
)


def test_models_list_returns_the_served_model(
    typesafe_client: TypeSafeClient,
) -> None:
    models = typesafe_client.models.list()

    assert [normpath(model.name) for model in models.models] == [KEV_0_8B_MODEL_PATH]
