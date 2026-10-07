export class DesiredStateServesAnotherModeError extends Error {
  constructor() {
    super("The desired state does not serve text generation");
    this.name = "DesiredStateServesAnotherModeError";
  }
}
