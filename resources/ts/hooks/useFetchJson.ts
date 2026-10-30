import { useEffect, useState } from "react";
import type { z } from "zod";

import { fetchJsonLoadingState } from "@intentee/paddler-client/FetchJsonLoadingState";
import { type FetchJsonState } from "@intentee/paddler-client/FetchJsonState";

export function useFetchJson<TResponseSchema extends z.ZodType>({
  produceFetchPromise,
  responseSchema,
}: {
  produceFetchPromise(this: void, abortSignal: AbortSignal): Promise<Response>;
  responseSchema: TResponseSchema;
}): FetchJsonState<z.infer<TResponseSchema>> {
  const [fetchState, setFetchState] = useState<
    FetchJsonState<z.infer<TResponseSchema>>
  >(fetchJsonLoadingState);
  const [requestedFetch, setRequestedFetch] = useState({
    produceFetchPromise,
    responseSchema,
  });

  if (
    requestedFetch.produceFetchPromise !== produceFetchPromise ||
    requestedFetch.responseSchema !== responseSchema
  ) {
    setRequestedFetch({ produceFetchPromise, responseSchema });
    setFetchState(fetchJsonLoadingState);
  }

  useEffect(
    function () {
      const abortController = new AbortController();

      produceFetchPromise(abortController.signal)
        .then(function (response) {
          if (!response.ok) {
            throw new Error(`HTTP error status: ${response.status}`);
          }

          return response.json();
        })
        .then(function (result: unknown) {
          return responseSchema.parse(result);
        })
        .then(function (result: z.infer<TResponseSchema>) {
          if (abortController.signal.aborted) {
            return;
          }

          setFetchState({
            empty: false,
            error: null,
            loading: false,
            ok: true,
            response: result,
          });
        })
        .catch(function (error: unknown) {
          if (abortController.signal.aborted) {
            return;
          }

          setFetchState({
            empty: false,
            error: String(error),
            loading: false,
            ok: false,
            response: null,
          });
        });

      return function () {
        abortController.abort("Component unmounted or fetch cancelled.");
      };
    },
    [produceFetchPromise, responseSchema, setFetchState],
  );

  return fetchState;
}
