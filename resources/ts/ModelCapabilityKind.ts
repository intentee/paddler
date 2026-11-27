import { type ModelCapabilities } from "@intentee/paddler-client/schemas/ModelCapabilities";

import { type ModelCapabilityStatus } from "./ModelCapabilityStatus";

export interface ModelCapabilityKind {
  label: string;
  statusOf(this: void, capabilities: ModelCapabilities): ModelCapabilityStatus;
}
