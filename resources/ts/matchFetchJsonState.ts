import { type ReactNode } from "react";

import { type FetchJsonEmptyState } from "@intentee/paddler-client/FetchJsonEmptyState";
import { type FetchJsonErrorState } from "@intentee/paddler-client/FetchJsonErrorState";
import { type FetchJsonLoadingState } from "@intentee/paddler-client/FetchJsonLoadingState";
import { type FetchJsonState } from "@intentee/paddler-client/FetchJsonState";
import { type FetchJsonSuccessState } from "@intentee/paddler-client/FetchJsonSuccessState";

interface Handlers<TResponse> {
  empty(state: FetchJsonEmptyState): ReactNode;
  error(state: FetchJsonErrorState): ReactNode;
  loading(state: FetchJsonLoadingState): ReactNode;
  ok(state: FetchJsonSuccessState<TResponse>): ReactNode;
}

export function matchFetchJsonState<TResponse>(
  state: FetchJsonState<TResponse>,
  handlers: Handlers<TResponse>,
): ReactNode {
  if (state.empty) {
    return handlers.empty(state);
  }

  if (state.loading) {
    return handlers.loading(state);
  }

  if (state.error !== null) {
    return handlers.error(state);
  }

  return handlers.ok(state);
}
