from paddler_client.agent_issue import AgentIssue


class PaddlerTestClusterError(Exception):
    pass


class AgentReportedIssuesError(PaddlerTestClusterError):
    def __init__(self, agent_name: str, issues: list[AgentIssue]) -> None:
        self.agent_name = agent_name
        self.issues = issues
        super().__init__(f"Agent {agent_name!r} reported issues: {issues!r}")


class AgentsStreamClosedError(PaddlerTestClusterError):
    def __init__(self, agent_name: str) -> None:
        self.agent_name = agent_name
        super().__init__(
            f"The agents stream closed before agent {agent_name!r} became ready"
        )


class BalancerExitedBeforeAnnouncingError(PaddlerTestClusterError):
    def __init__(self, returncode: int) -> None:
        self.returncode = returncode
        super().__init__(
            f"The balancer exited with status {returncode} before announcing "
            "its addresses"
        )


class CompatOpenAIServiceNotServedError(PaddlerTestClusterError):
    def __init__(self) -> None:
        super().__init__("The balancer does not serve the OpenAI compatibility service")


class ProcessDidNotExitError(PaddlerTestClusterError):
    def __init__(self, pid: int) -> None:
        self.pid = pid
        super().__init__(f"Process {pid} did not exit within Paddler's shutdown bound")


class ProcessExitedWithFailureError(PaddlerTestClusterError):
    def __init__(self, pid: int, returncode: int) -> None:
        self.pid = pid
        self.returncode = returncode
        super().__init__(f"Process {pid} exited with status {returncode}")
