export type WebSocketConnectingState = {
  isConnected: false;
  isConnectionClosed: false;
  isConnectionError: false;
  webSocket: null;
};

export const webSocketConnectingState: WebSocketConnectingState = Object.freeze(
  {
    isConnected: false,
    isConnectionClosed: false,
    isConnectionError: false,
    webSocket: null,
  },
);
