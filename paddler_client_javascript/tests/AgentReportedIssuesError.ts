import type { AgentIssue } from "../src/schemas/AgentIssue";

export class AgentReportedIssuesError extends Error {
  override name = "AgentReportedIssuesError";

  constructor(
    public readonly agentName: string,
    public readonly issues: ReadonlyArray<AgentIssue>,
  ) {
    super(`Agent ${agentName} reported issues: ${JSON.stringify(issues)}`);
  }
}
