from contextlib import AsyncExitStack

import pytest
from paddler_client.agent_desired_model import AgentDesiredModel
from paddler_client.balancer_desired_state import BalancerDesiredState

from paddler_test_cluster.agent_spec import AgentSpec
from paddler_test_cluster.error import AgentReportedIssuesError
from paddler_test_cluster.paddler_cluster import paddler_cluster


async def test_agent_reporting_issues_fails_the_cluster() -> None:
    async with AsyncExitStack() as exit_stack:
        with pytest.raises(AgentReportedIssuesError) as reported_issues:
            await exit_stack.enter_async_context(
                paddler_cluster(
                    BalancerDesiredState(
                        model=AgentDesiredModel.local_to_agent(
                            "/nonexistent/model.gguf"
                        ),
                    ),
                    [AgentSpec(name="agent-without-its-model", slots=1)],
                )
            )

    assert reported_issues.value.agent_name == "agent-without-its-model"
    assert [issue.variant for issue in reported_issues.value.issues] == [
        "ModelFileDoesNotExist"
    ]
