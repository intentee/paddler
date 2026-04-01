from paddler_client.agent_controller_pool_snapshot import (
    AgentControllerPoolSnapshot,
)
from paddler_client.agent_controller_snapshot import AgentControllerSnapshot
from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)


def test_agent_controller_snapshot_deserialization() -> None:
    snapshot = AgentControllerSnapshot.model_validate(
        {
            "desired_slots_total": 4,
            "download_current": 100,
            "download_filename": "model.gguf",
            "download_total": 1000,
            "id": "agent-1",
            "issues": [{"SlotCannotStart": {"error": "OOM", "slot_index": 0}}],
            "model_path": "/models/test.gguf",
            "name": "my-agent",
            "slots_processing": 1,
            "slots_total": 4,
            "state_application_status": "Fresh",
            "uses_chat_template_override": True,
        }
    )

    assert snapshot.id == "agent-1"
    assert snapshot.download_filename == "model.gguf"
    assert len(snapshot.issues) == 1
    assert snapshot.issues[0].variant == "SlotCannotStart"
    assert snapshot.state_application_status == AgentStateApplicationStatus.FRESH


def test_agent_controller_pool_snapshot_deserialization() -> None:
    pool = AgentControllerPoolSnapshot.model_validate(
        {
            "agents": [
                {
                    "desired_slots_total": 2,
                    "download_current": 0,
                    "download_total": 0,
                    "id": "a1",
                    "issues": [],
                    "slots_processing": 0,
                    "slots_total": 2,
                    "state_application_status": "Applied",
                    "uses_chat_template_override": False,
                }
            ]
        }
    )

    assert len(pool.agents) == 1
    assert pool.agents[0].id == "a1"
