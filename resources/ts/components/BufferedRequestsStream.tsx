import React, { useContext } from "react";

import { BufferedRequestsResponseSchema } from "@intentee/paddler-client/schemas/BufferedRequestsResponse";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useEventSourceUpdates } from "../hooks/useEventSourceUpdates";
import { matchEventSourceUpdateState } from "../matchEventSourceUpdateState";
import { BufferedRequests } from "./BufferedRequests";

import { dashboardSectionStreamLoader } from "./dashboardSectionStreamLoader.module.css";

export function BufferedRequestsStream() {
  const { managementAddr } = useContext(PaddlerConfigurationContext);

  const eventSourceUpdateState = useEventSourceUpdates({
    schema: BufferedRequestsResponseSchema,
    endpoint: `//${managementAddr}/api/v1/buffered_requests/stream`,
  });

  return matchEventSourceUpdateState(eventSourceUpdateState, {
    connected() {
      return (
        <div className={dashboardSectionStreamLoader}>
          Connected to the server, waiting for buffered requests update...
        </div>
      );
    },
    connectionError() {
      return (
        <div className={dashboardSectionStreamLoader}>
          Cannot connect to the server to get the buffered requests updates
          stream. Will try to reconnect in a few seconds...
        </div>
      );
    },
    dataSnapshot({ data: { buffered_requests_current } }) {
      return (
        <BufferedRequests currentBufferedRequests={buffered_requests_current} />
      );
    },
    deserializationError() {
      return (
        <div className={dashboardSectionStreamLoader}>
          Error deserializing buffered requests data from the server.
        </div>
      );
    },
    initial() {
      return (
        <div className={dashboardSectionStreamLoader}>
          Connecting to the server...
        </div>
      );
    },
  });
}
