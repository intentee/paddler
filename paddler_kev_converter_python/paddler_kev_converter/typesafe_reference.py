import json
from pathlib import Path
from typing import Any

from kev.api import SystemOneRequest, to_answers, to_record

REFERENCE_CASES: list[dict[str, Any]] = [
    {
        "request": {
            "state": "The invoice was paid on time.",
            "questions": {
                "paid": {"type": "noul", "instructions": "Was the invoice paid?"},
            },
        },
        "probabilities": [[0.12345678, 0.87654322]],
    },
    {
        "request": {
            "state": {
                "customer": "Ada",
                "orders": [
                    {"id": 1, "total": 19.99, "shipped": True},
                    {"id": 2, "total": 100.0, "shipped": False, "notes": None},
                ],
                "tags": [],
                "metadata": {},
                "balance": -0.0,
                "ratio": 1e-07,
                "population": 1e16,
                "huge": 1.5e300,
                "count": 123456789012345678,
            },
            "questions": {
                "sentiment": {
                    "type": "choice",
                    "instructions": {"task": "Classify", "hints": ["be brief", 2]},
                    "criteria": {
                        "positive": "The customer is happy",
                        "neutral": None,
                        "negative": "",
                        "mixed": {"when": "both", "weight": 0.5},
                    },
                },
                "risk": {
                    "type": "score",
                    "instructions": None,
                    "criteria": [
                        "no risk",
                        {"level": "some", "detail": ["late", "partial"]},
                        "high risk",
                    ],
                },
                "vip": {
                    "type": "noul",
                    "instructions": 42,
                    "criteria": {"true": "Spends a lot", "false": {"reason": "rare"}},
                },
            },
        },
        "probabilities": [
            [0.25, 0.25, 0.4, 0.1],
            [0.2, 0.3, 0.5],
            [0.5, 0.5],
        ],
    },
    {
        "request": {
            "state": ["first line\nsecond line", 3.5, True, None, ["nested", 1]],
            "questions": {
                "tie": {
                    "type": "choice",
                    "instructions": ["pick", "one"],
                    "criteria": {"left": 1, "right": False},
                },
                "single": {
                    "type": "score",
                    "instructions": 2.5,
                    "criteria": ["only level"],
                },
            },
        },
        "probabilities": [[0.5, 0.5], [1.0]],
    },
    {
        "request": {
            "state": (
                "Za\u017c\u00f3\u0142\u0107 g\u0119\u015bl\u0105 ja\u017a\u0144 "
                "\U0001f468\u200d\U0001f469"
            ),
            "questions": {
                "lonely": {
                    "type": "choice",
                    "instructions": True,
                    "criteria": {"only": "the only option"},
                },
                "uncertain": {
                    "type": "score",
                    "instructions": "How confident?",
                    "criteria": ["low", "medium", "high", "certain"],
                },
            },
        },
        "probabilities": [[1.0], [0.25, 0.25, 0.25, 0.25]],
    },
    {
        "request": {
            "state": {
                "precise": 123.45678901234567,
                "halfway": 1000000000000.6562,
                "power_of_two": 7.120236347223045e-307,
            },
            "questions": {
                "measure": {
                    "type": "score",
                    "instructions": 1000000000000.6562,
                    "criteria": [123.45678901234567, 7.120236347223045e-307],
                },
                "pick": {
                    "type": "choice",
                    "instructions": "Which value is exact?",
                    "criteria": {
                        "halfway": 1000000000000.6562,
                        "precise": 123.45678901234567,
                    },
                },
            },
        },
        "probabilities": [[0.5, 0.5], [0.5, 0.5]],
    },
]


def _rendered(record: dict[str, Any]) -> dict[str, Any]:
    return {
        "state": record["state"],
        "questions": [
            {"instructions": question["instr"], "options": question["options"]}
            for question in record["questions"]
        ],
    }


def write_typesafe_reference(path: Path) -> None:
    references = []

    for reference_case in REFERENCE_CASES:
        record, question_metadata = to_record(
            SystemOneRequest.model_validate(reference_case["request"])
        )
        references.append(
            {
                "request": reference_case["request"],
                "rendered": _rendered(record),
                "probabilities": reference_case["probabilities"],
                "answers": to_answers(
                    reference_case["probabilities"], question_metadata
                ),
            }
        )

    path.write_text(
        json.dumps(references, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
