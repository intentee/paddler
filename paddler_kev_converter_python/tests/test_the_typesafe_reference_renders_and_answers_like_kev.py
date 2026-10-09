import json
from pathlib import Path

from paddler_kev_converter.command_line import main


def test_the_typesafe_reference_renders_and_answers_like_kev(tmp_path: Path) -> None:
    main(["typesafe-reference", "--output", str(tmp_path / "reference.json")])
    reference = json.loads((tmp_path / "reference.json").read_text(encoding="utf-8"))

    assert reference[0] == {
        "request": {
            "state": "The invoice was paid on time.",
            "questions": {
                "paid": {"type": "noul", "instructions": "Was the invoice paid?"},
            },
        },
        "rendered": {
            "state": "The invoice was paid on time.",
            "questions": [
                {"instructions": "Was the invoice paid?", "options": ["no", "yes"]}
            ],
        },
        "probabilities": [[0.12345678, 0.87654322]],
        "answers": {"paid": {"type": "noul", "noul": 0.8765}},
    }
