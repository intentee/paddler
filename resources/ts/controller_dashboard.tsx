import React from "react";
import { createRoot } from "react-dom/client";

import { type StatsdConfiguration } from "./StatsdConfiguration";
import { Home } from "./components/Home";
import { PaddlerConfigurationContext } from "./contexts/PaddlerConfigurationContext";

class RootNode {
  constructor(private rootNodeElement: HTMLElement) {}

  getIntFromDataset(key: string): number {
    return parseInt(this.getStringFromDataset(key), 10);
  }

  getOptionalStringFromDataset(key: string): string | null {
    return this.rootNodeElement.dataset[key] ?? null;
  }

  getStatsdConfiguration(): StatsdConfiguration | null {
    const addr = this.getOptionalStringFromDataset("statsdAddr");

    if (addr === null) {
      return null;
    }

    return {
      addr,
      prefix: this.getStringFromDataset("statsdPrefix"),
      reportingIntervalMillis: this.getIntFromDataset(
        "statsdReportingIntervalMillis",
      ),
    };
  }

  getStringFromDataset(key: string): string {
    const value = this.getOptionalStringFromDataset(key);

    if (value === null) {
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
      compatOpenAIAddr:
        rootNode.getOptionalStringFromDataset("compatOpenaiAddr"),
      compatTypeSafeAddr:
        rootNode.getOptionalStringFromDataset("compatTypesafeAddr"),
      inferenceAddr: rootNode.getStringFromDataset("inferenceAddr"),
      managementAddr: rootNode.getStringFromDataset("managementAddr"),
      maxBufferedRequests: rootNode.getIntFromDataset("maxBufferedRequests"),
      statsd: rootNode.getStatsdConfiguration(),
    }}
  >
    <Home />
  </PaddlerConfigurationContext.Provider>,
);
