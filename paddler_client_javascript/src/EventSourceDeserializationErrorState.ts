export type EventSourceDeserializationErrorState = {
  data: undefined;
  isConnected: true;
  isConnectionError: false;
  isDeserializationError: true;
  isInitial: false;
  isOk: false;
};

export const eventSourceDeserializationErrorState: EventSourceDeserializationErrorState =
  Object.freeze({
    data: undefined,
    isConnected: true,
    isConnectionError: false,
    isDeserializationError: true,
    isInitial: false,
    isOk: false,
  });
