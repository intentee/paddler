export type EventSourceInitialState = {
  data: undefined;
  isConnected: false;
  isConnectionError: false;
  isDeserializationError: false;
  isInitial: true;
  isOk: false;
};

export const eventSourceInitialState: EventSourceInitialState = Object.freeze({
  data: undefined,
  isConnected: false,
  isConnectionError: false,
  isDeserializationError: false,
  isInitial: true,
  isOk: false,
});
