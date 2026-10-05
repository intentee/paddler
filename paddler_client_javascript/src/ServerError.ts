import { PaddlerError } from "./PaddlerError";

export class ServerError extends PaddlerError {
  override name = "ServerError";

  constructor(
    public readonly code: number,
    message: string,
  ) {
    super(message);
  }
}
