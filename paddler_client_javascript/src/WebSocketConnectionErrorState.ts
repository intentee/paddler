export type WebSocketConnectionErrorState = {
  isConnected: false;
  isConnectionClosed: false;
  isConnectionError: true;
  webSocket: null;
};

export const webSocketConnectionErrorState: WebSocketConnectionErrorState =
  Object.freeze({
    isConnected: false,
    isConnectionClosed: false,
    isConnectionError: true,
    webSocket: null,
  });
