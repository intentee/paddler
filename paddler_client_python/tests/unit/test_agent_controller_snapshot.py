from paddler_client.agent_controller_snapshot import AgentControllerSnapshot
from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)


def test_agent_controller_snapshot_deserialization() -> None:
    snapshot = AgentControllerSnapshot.model_validate(
        {
            "id": "agent-1",
            "name": "my-agent",
            "slots_processing": 1,
            "status": {
                "desired_slots_total": 4,
                "download_status": "NotDownloading",
                "issues": [{"SlotCannotStart": {"error": "OOM", "slot_index": 0}}],
                "model_path": "/models/test.gguf",
                "slots_total": 4,
                "state_application_status": "Fresh",
                "uses_chat_template_override": True,
            },
        }
    )

    assert snapshot.id == "agent-1"
    assert [issue.variant for issue in snapshot.status.issues] == ["SlotCannotStart"]
    assert snapshot.status.state_application_status == AgentStateApplicationStatus.FRESH
