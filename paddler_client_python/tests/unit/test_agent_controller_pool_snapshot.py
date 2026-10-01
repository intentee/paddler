from paddler_client.agent_controller_pool_snapshot import (
    AgentControllerPoolSnapshot,
)


def test_agent_controller_pool_snapshot_reads_every_agent() -> None:
    pool = AgentControllerPoolSnapshot.model_validate(
        {
            "agents": [
                {
                    "id": "a1",
                    "slots_processing": 0,
                    "status": {
                        "desired_slots_total": 2,
                        "download_status": "NotDownloading",
                        "slots_total": 2,
                        "state_application_status": "Applied",
                        "uses_chat_template_override": False,
                    },
                }
            ]
        }
    )

    assert len(pool.agents) == 1
    assert pool.agents[0].id == "a1"
