import { createContext } from "react";

import { type InferenceMode } from "@intentee/paddler-client/schemas/InferenceMode";
import { type StatsdConfiguration } from "../StatsdConfiguration";

export type PaddlerConfigurationContextValue = {
  bufferedRequestTimeoutMillis: number;
  compatOpenAIAddr: string | null;
  compatTypeSafeAddr: string | null;
  inferenceAddr: string;
  inferenceMode: InferenceMode;
  managementAddr: string;
  maxBufferedRequests: number;
  statsd: StatsdConfiguration | null;
};

export const PaddlerConfigurationContext =
  createContext<PaddlerConfigurationContextValue>({
    get bufferedRequestTimeoutMillis(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get compatOpenAIAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get compatTypeSafeAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get inferenceAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get inferenceMode(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get managementAddr(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get maxBufferedRequests(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
    get statsd(): never {
      throw new Error("PaddlerConfigurationContext not provided");
    },
  });
