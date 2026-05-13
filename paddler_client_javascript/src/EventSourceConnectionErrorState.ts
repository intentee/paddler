export type EventSourceConnectionErrorState = {
  data: undefined;
  isConnected: false;
  isConnectionError: true;
  isDeserializationError: false;
  isInitial: false;
  isOk: false;
};

export const eventSourceConnectionErrorState: EventSourceConnectionErrorState =
  Object.freeze({
    data: undefined,
    isConnected: false,
    isConnectionError: true,
    isDeserializationError: false,
    isInitial: false,
    isOk: false,
  });
