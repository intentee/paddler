import { z } from "zod";

import { ChatTemplateCapabilitiesSchema } from "./ChatTemplateCapabilities";
import { ConversationUnsupportedReasonSchema } from "./ConversationUnsupportedReason";

export const ConversationCapabilitySchema = z.union([
  z.object({ Supported: ChatTemplateCapabilitiesSchema }).strict(),
  z.object({ Unsupported: ConversationUnsupportedReasonSchema }).strict(),
]);

export type ConversationCapability = z.infer<
  typeof ConversationCapabilitySchema
>;
