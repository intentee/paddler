export class BalancerExitedBeforeAnnouncingError extends Error {
  override name = "BalancerExitedBeforeAnnouncingError";

  constructor(public readonly exitCode: number | null) {
    super(
      `The balancer exited with code ${String(exitCode)} before announcing its addresses`,
    );
  }
}
