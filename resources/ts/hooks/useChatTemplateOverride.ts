import { useCallback } from "react";

import { ChatTemplateSchema } from "@intentee/paddler-client/schemas/ChatTemplate";
import { useFetchJson } from "./useFetchJson";

const responseSchema = ChatTemplateSchema.nullable();

export function useChatTemplateOverride({
  agentId,
  managementAddr,
}: {
  agentId: string;
  managementAddr: string;
}) {
  const produceFetchPromise = useCallback(
    function (signal: AbortSignal) {
      return fetch(
        `//${managementAddr}/api/v1/agent/${agentId}/chat_template_override`,
        {
          signal,
        },
      );
    },
    [agentId, managementAddr],
  );

  const result = useFetchJson({
    produceFetchPromise,
    responseSchema,
  });

  return result;
}
