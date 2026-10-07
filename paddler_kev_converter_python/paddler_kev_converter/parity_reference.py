import json
from pathlib import Path
from typing import Any

from kev.api import SystemOneRequest, to_record
from kev.checkpoint import Checkpoint, LoadOptions

PARITY_REQUESTS: list[dict[str, Any]] = [
    {
        "state": (
            "Shoes arrived two weeks late and in the wrong size. "
            "Also I see two charges on my card."
        ),
        "questions": {
            "department": {
                "type": "choice",
                "instructions": "Which team should handle this?",
                "criteria": {
                    "returns": "Exchanges, refunds, wrong or damaged items",
                    "shipping": "Delivery status, delays, lost packages",
                    "billing": "Charges, invoices, payment problems",
                },
            },
            "escalate": {
                "type": "noul",
                "instructions": "Does this need urgent human attention?",
            },
            "frustration": {
                "type": "score",
                "instructions": "How frustrated is the customer?",
                "criteria": ["Calm", "Frustrated", "Very angry"],
            },
        },
    },
    {
        "state": "I was charged twice. Please help.",
        "questions": {
            "billing": {"type": "noul", "instructions": "Is this about billing?"},
            "tone": {
                "type": "choice",
                "instructions": "What is the tone?",
                "criteria": {"calm": None, "angry": None},
            },
        },
    },
    {
        "state": {
            "customer": "Ada",
            "message": "My package never arrived and nobody answers my emails.",
        },
        "questions": {
            "lost": {"type": "noul", "instructions": "Is the package lost?"},
            "urgency": {
                "type": "score",
                "instructions": "How urgent is the follow-up?",
                "criteria": [
                    "Can wait",
                    "Needs attention this week",
                    "Needs attention today",
                ],
            },
        },
    },
    {
        "state": (
            "Premise: A man is playing a guitar on stage. "
            "Hypothesis: A person is performing music."
        ),
        "questions": {
            "relation": {
                "type": "choice",
                "instructions": "Does the premise entail the hypothesis?",
                "criteria": {
                    "entailment": "The hypothesis follows from the premise",
                    "neutral": "The premise neither supports nor contradicts it",
                    "contradiction": "The hypothesis contradicts the premise",
                },
            },
        },
    },
]


def write_parity_reference(checkpoint_reference: str, path: Path) -> None:
    tokenizer, model = Checkpoint(checkpoint_reference).load("cpu", LoadOptions())
    records = []

    for request in PARITY_REQUESTS:
        record, question_metadata = to_record(SystemOneRequest.model_validate(request))
        probabilities = model.probs(model.encode(tokenizer, record, strict=True))
        records.append(
            {
                "state": record["state"],
                "questions": [
                    {
                        "id": metadata["id"],
                        "instructions": question["instr"],
                        "options": question["options"],
                    }
                    for question, metadata in zip(
                        record["questions"], question_metadata, strict=True
                    )
                ],
                "probabilities": [
                    [float(probability) for probability in question_probabilities]
                    for question_probabilities in probabilities
                ],
            }
        )

    path.write_text(
        json.dumps(records, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
