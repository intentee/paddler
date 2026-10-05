import { createContext } from "react";

export type PaddlerConfigurationContextValue = {
  bufferedRequestTimeoutMillis: number;
  compatOpenAIAddr: string;
  inferenceAddr: string;
  managementAddr: string;
  maxBufferedRequests: number;
  statsdAddr: string;
  statsdPrefix: string;
  statsdReportingIntervalMillis: number;
};

export const PaddlerConfigurationContext =
  createContext<PaddlerConfigurationContextValue>({
    get bufferedRequestTimeoutMillis(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get compatOpenAIAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get inferenceAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get managementAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get maxBufferedRequests(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get statsdAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get statsdPrefix(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get statsdReportingIntervalMillis(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
  });
