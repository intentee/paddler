from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)


def test_agent_state_application_status_values() -> None:
    assert AgentStateApplicationStatus.APPLIED.value == "Applied"
    assert (
        AgentStateApplicationStatus.ATTEMPTED_AND_NOT_APPLIABLE.value
        == "AttemptedAndNotAppliable"
    )
    assert AgentStateApplicationStatus.FRESH.value == "Fresh"
    assert AgentStateApplicationStatus.STUCK.value == "Stuck"
