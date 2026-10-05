export class AgentsStreamClosedError extends Error {
  override name = "AgentsStreamClosedError";

  constructor(public readonly agentName: string) {
    super(`The agents stream closed before agent ${agentName} became ready`);
  }
}
