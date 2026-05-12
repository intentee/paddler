import { useEffect, useState } from "react";
import type { z } from "zod";

import { fetchJsonEmptyState } from "@intentee/paddler-client/FetchJsonEmptyState";
import { fetchJsonLoadingState } from "@intentee/paddler-client/FetchJsonLoadingState";
import { type FetchJsonState } from "@intentee/paddler-client/FetchJsonState";

export function useFetchJson<TResponseSchema extends z.ZodType>({
  produceFetchPromise,
  responseSchema,
}: {
  produceFetchPromise(
    this: void,
    abortSignal: AbortSignal,
  ): null | Promise<Response>;
  responseSchema: TResponseSchema;
}): FetchJsonState<z.infer<TResponseSchema>> {
  const [fetchState, setFetchState] = useState<
    FetchJsonState<z.infer<TResponseSchema>>
  >(fetchJsonLoadingState);

  useEffect(
    function () {
      const abortController = new AbortController();
      const fetchPromise = produceFetchPromise(abortController.signal);

      if (!fetchPromise) {
        setFetchState(fetchJsonEmptyState);

        return function () {
          abortController.abort("Fetch promise was not provided.");
        };
      }

      setFetchState(fetchJsonLoadingState);

      fetchPromise
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
          setFetchState({
            empty: false,
            error: null,
            loading: false,
            ok: true,
            response: result,
          });
        })
        .catch(function (error: unknown) {
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
