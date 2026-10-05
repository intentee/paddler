import { useCallback } from "react";
import { z } from "zod";

import { useFetchJson } from "./useFetchJson";

const responseSchema = z
  .object({
    metadata: z.record(z.string(), z.string()),
  })
  .strict()
  .nullable();

export function useModelMetadata({
  agentId,
  managementAddr,
}: {
  agentId: string;
  managementAddr: string;
}) {
  const produceFetchPromise = useCallback(
    function (signal: AbortSignal) {
      return fetch(
        `//${managementAddr}/api/v1/agent/${agentId}/model_metadata`,
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
