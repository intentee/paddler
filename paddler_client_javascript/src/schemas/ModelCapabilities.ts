import { z } from "zod";

import { ConversationCapabilitySchema } from "./ConversationCapability";
import { DecisionsCapabilitySchema } from "./DecisionsCapability";
import { EmbeddingsCapabilitySchema } from "./EmbeddingsCapability";
import { InputMediaCapabilitiesSchema } from "./InputMediaCapabilities";
import { ModelDefaultsSchema } from "./ModelDefaults";
import { TextGenerationCapabilitySchema } from "./TextGenerationCapability";

export const ModelCapabilitiesSchema = z
  .object({
    conversation: ConversationCapabilitySchema,
    decisions: DecisionsCapabilitySchema,
    embeddings: EmbeddingsCapabilitySchema,
    input_media: InputMediaCapabilitiesSchema,
    model_defaults: ModelDefaultsSchema,
    text_generation: TextGenerationCapabilitySchema,
  })
  .strict();

export type ModelCapabilities = z.infer<typeof ModelCapabilitiesSchema>;
