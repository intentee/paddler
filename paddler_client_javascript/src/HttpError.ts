import { PaddlerError } from "./PaddlerError";

export class HttpError extends PaddlerError {
  override name = "HttpError";

  constructor(
    public readonly statusCode: number,
    message: string,
  ) {
    super(message);
  }
}
