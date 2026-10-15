import json
from math import isclose
from pathlib import Path

from paddler_kev_converter.command_line import main
from tests.conftest import KEV_CHECKPOINT


def test_the_parity_reference_pairs_rendered_records_with_kev_probabilities(
    tmp_path: Path,
) -> None:
    main(
        [
            "parity-reference",
            "--checkpoint",
            KEV_CHECKPOINT,
            "--output",
            str(tmp_path / "parity.json"),
        ]
    )
    records = json.loads((tmp_path / "parity.json").read_text(encoding="utf-8"))

    assert records[1]["state"] == "I was charged twice. Please help."
    assert records[1]["questions"] == [
        {
            "id": "billing",
            "instructions": "Is this about billing?",
            "options": ["no", "yes"],
        },
        {
            "id": "tone",
            "instructions": "What is the tone?",
            "options": ["calm", "angry"],
        },
    ]
    assert all(
        len(question_probabilities) == len(question["options"])
        and isclose(sum(question_probabilities), 1.0, rel_tol=1e-5)
        for record in records
        for question, question_probabilities in zip(
            record["questions"], record["probabilities"], strict=True
        )
    )
