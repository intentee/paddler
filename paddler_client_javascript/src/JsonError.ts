import { PaddlerError } from "./PaddlerError";

export class JsonError extends PaddlerError {
  override name = "JsonError";

  constructor(
    message: string,
    public readonly raw: string,
  ) {
    super(message);
  }
}
