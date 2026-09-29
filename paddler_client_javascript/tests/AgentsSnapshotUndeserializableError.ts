export class AgentsSnapshotUndeserializableError extends Error {
  override name = "AgentsSnapshotUndeserializableError";

  constructor(public readonly agentName: string) {
    super(
      `The agents stream sent a snapshot that could not be deserialized while waiting for agent ${agentName}`,
    );
  }
}
