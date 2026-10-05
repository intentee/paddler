import React from "react";
import { createRoot } from "react-dom/client";

import { Home } from "./components/Home";
import { PaddlerConfigurationContext } from "./contexts/PaddlerConfigurationContext";

class RootNode {
  constructor(private rootNodeElement: HTMLElement) {}

  getIntFromDataset(key: string): number {
    return parseInt(this.getStringFromDataset(key), 10);
  }

  getStringFromDataset(key: string): string {
    const value = this.rootNodeElement.dataset[key];

    if (value === undefined) {
      throw new Error(`Missing dataset key: ${key}`);
    }

    return value;
  }
}

const rootNodeElement = document.getElementById("paddler-dashboard");

if (!rootNodeElement) {
  throw new Error("Root node not found");
}

const rootNode = new RootNode(rootNodeElement);

const root = createRoot(rootNodeElement);

root.render(
  <PaddlerConfigurationContext.Provider
    value={{
      bufferedRequestTimeoutMillis: rootNode.getIntFromDataset(
        "bufferedRequestTimeoutMillis",
      ),
      compatOpenAIAddr: rootNode.getStringFromDataset("compatOpenaiAddr"),
      inferenceAddr: rootNode.getStringFromDataset("inferenceAddr"),
      managementAddr: rootNode.getStringFromDataset("managementAddr"),
      maxBufferedRequests: rootNode.getIntFromDataset("maxBufferedRequests"),
      statsdAddr: rootNode.getStringFromDataset("statsdAddr"),
      statsdPrefix: rootNode.getStringFromDataset("statsdPrefix"),
      statsdReportingIntervalMillis: rootNode.getIntFromDataset(
        "statsdReportingIntervalMillis",
      ),
    }}
  >
    <Home />
  </PaddlerConfigurationContext.Provider>,
);
