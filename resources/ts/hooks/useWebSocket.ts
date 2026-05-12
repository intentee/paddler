import { useEffect, useRef, useState } from "react";

import { webSocketConnectingState } from "@intentee/paddler-client/WebSocketConnectingState";
import { webSocketConnectionClosedState } from "@intentee/paddler-client/WebSocketConnectionClosedState";
import { webSocketConnectionErrorState } from "@intentee/paddler-client/WebSocketConnectionErrorState";
import { type WebSocketState } from "@intentee/paddler-client/WebSocketState";

const MAX_RECONNECT_DEBOUNCE_TIME_INCREASE = 3;
const RECONNECT_DELAY = 600;

function incrementVersion(version: number): number {
  return version + 1;
}

export function useWebSocket({
  endpoint,
}: {
  endpoint: string;
}): WebSocketState {
  const [socketState, setSocketState] = useState<WebSocketState>(
    webSocketConnectingState,
  );
  const [version, setVersion] = useState(0);
  const [webSocket, setWebSocket] = useState<null | WebSocket>(null);
  const reconnectAttempts = useRef(0);

  useEffect(
    function () {
      function connect() {
        const webSocket = new WebSocket(endpoint);

        setWebSocket(webSocket);
      }

      if (version < 1) {
        connect();

        return;
      }

      reconnectAttempts.current += 1;

      const timeoutId = setTimeout(
        connect,
        Math.min(
          reconnectAttempts.current,
          MAX_RECONNECT_DEBOUNCE_TIME_INCREASE,
        ) * RECONNECT_DELAY,
      );

      return function () {
        clearTimeout(timeoutId);
      };
    },
    [endpoint, setWebSocket, version],
  );

  useEffect(
    function () {
      if (!webSocket) {
        return;
      }

      return function () {
        webSocket.close();
      };
    },
    [webSocket],
  );

  useEffect(
    function () {
      if (!webSocket) {
        return;
      }

      webSocket.addEventListener("close", function () {
        setSocketState(webSocketConnectionClosedState);
        setVersion(incrementVersion);
      });

      webSocket.addEventListener("error", function (event) {
        console.error("WebSocket error:", event);
        setSocketState(webSocketConnectionErrorState);
        setVersion(incrementVersion);
      });

      webSocket.addEventListener("open", function () {
        reconnectAttempts.current = 0;

        setSocketState({
          isConnected: true,
          isConnectionClosed: false,
          isConnectionError: false,
          webSocket: webSocket,
        });
      });
    },
    [endpoint, setSocketState, setVersion, webSocket],
  );

  return socketState;
}
