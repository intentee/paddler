export class ProcessExitedWithFailureError extends Error {
  override name = "ProcessExitedWithFailureError";

  constructor(
    public readonly pid: number,
    public readonly exitCode: number | null,
    public readonly signalCode: NodeJS.Signals | null,
  ) {
    super(
      `Process ${pid} exited with code ${String(exitCode)} and signal ${String(signalCode)}`,
    );
  }
}
