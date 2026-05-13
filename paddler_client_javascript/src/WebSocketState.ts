import type { WebSocketConnectingState } from "./WebSocketConnectingState";
import type { WebSocketConnectionClosedState } from "./WebSocketConnectionClosedState";
import type { WebSocketConnectionErrorState } from "./WebSocketConnectionErrorState";
import type { WebSocketConnectionOpenedState } from "./WebSocketConnectionOpenedState";

export type WebSocketState =
  | WebSocketConnectingState
  | WebSocketConnectionClosedState
  | WebSocketConnectionErrorState
  | WebSocketConnectionOpenedState;
