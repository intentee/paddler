import { PaddlerError } from "./PaddlerError";

export class ConnectionDroppedError extends PaddlerError {
  override name = "ConnectionDroppedError";

  constructor(public readonly requestId: string) {
    super(`Connection dropped while streaming request ${requestId}`);
  }
}
