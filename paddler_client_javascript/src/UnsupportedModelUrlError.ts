import { PaddlerError } from "./PaddlerError";

export class UnsupportedModelUrlError extends PaddlerError {
  override name = "UnsupportedModelUrlError";

  constructor(public readonly url: string) {
    super("Unsupported URL format");
  }
}
