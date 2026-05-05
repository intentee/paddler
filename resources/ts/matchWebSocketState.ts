import { type ReactNode } from "react";

import { type WebSocketConnectingState } from "@intentee/paddler-client/WebSocketConnectingState";
import { type WebSocketConnectionClosedState } from "@intentee/paddler-client/WebSocketConnectionClosedState";
import { type WebSocketConnectionErrorState } from "@intentee/paddler-client/WebSocketConnectionErrorState";
import { type WebSocketConnectionOpenedState } from "@intentee/paddler-client/WebSocketConnectionOpenedState";
import { type WebSocketState } from "@intentee/paddler-client/WebSocketState";

interface Handlers {
  connected(state: WebSocketConnectionOpenedState): ReactNode;
  connecting(state: WebSocketConnectingState): ReactNode;
  connectionClosed(state: WebSocketConnectionClosedState): ReactNode;
  connectionError(state: WebSocketConnectionErrorState): ReactNode;
}

export function matchWebSocketState(
  socketState: WebSocketState,
  handlers: Handlers,
): ReactNode {
  if (socketState.isConnected) {
    return handlers.connected(socketState);
  }

  if (socketState.isConnectionClosed) {
    return handlers.connectionClosed(socketState);
  }

  if (socketState.isConnectionError) {
    return handlers.connectionError(socketState);
  }

  return handlers.connecting(socketState);
}
