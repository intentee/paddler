export type WebSocketConnectionClosedState = {
  isConnected: false;
  isConnectionClosed: true;
  isConnectionError: false;
  webSocket: null;
};

export const webSocketConnectionClosedState: WebSocketConnectionClosedState =
  Object.freeze({
    isConnected: false,
    isConnectionClosed: true,
    isConnectionError: false,
    webSocket: null,
  });
