export type EventSourceConnectedState = {
  data: undefined;
  isConnected: true;
  isConnectionError: false;
  isDeserializationError: false;
  isInitial: false;
  isOk: false;
};

export const eventSourceConnectedState: EventSourceConnectedState =
  Object.freeze({
    data: undefined,
    isConnected: true,
    isConnectionError: false,
    isDeserializationError: false,
    isInitial: false,
    isOk: false,
  });
