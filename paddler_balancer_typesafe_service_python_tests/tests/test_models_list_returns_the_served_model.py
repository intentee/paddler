from json import load
from os import environ
from urllib.request import urlopen

from typesafe_sdk import TypeSafeClient


def served_model_paths() -> list[str]:
    with urlopen(f"{environ['PADDLER_MANAGEMENT_URL']}/api/v1/agents") as agents_response:
        agents_snapshot: dict[str, list[dict[str, dict[str, str]]]] = load(agents_response)

    return [agent["status"]["model_path"] for agent in agents_snapshot["agents"]]


def test_models_list_returns_the_served_model(
    typesafe_client: TypeSafeClient,
) -> None:
    models = typesafe_client.models.list()

    assert [model.name for model in models.models] == served_model_paths()
