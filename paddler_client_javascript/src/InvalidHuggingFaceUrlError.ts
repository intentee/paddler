import { PaddlerError } from "./PaddlerError";

export class InvalidHuggingFaceUrlError extends PaddlerError {
  override name = "InvalidHuggingFaceUrlError";

  constructor(public readonly pathname: string) {
    super(`Invalid Hugging Face URL format: ${pathname}`);
  }
}
