export class AgentExitedBeforeReadyError extends Error {
  override name = "AgentExitedBeforeReadyError";

  constructor(
    public readonly agentName: string,
    public readonly exitCode: number | null,
  ) {
    super(
      `Agent ${agentName} exited with code ${String(exitCode)} before it was ready`,
    );
  }
}
