import { PaddlerError } from "./PaddlerError";

export class WebSocketError extends PaddlerError {
  override name = "WebSocketError";
}
