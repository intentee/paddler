import type { FetchJsonEmptyState } from "./FetchJsonEmptyState";
import type { FetchJsonErrorState } from "./FetchJsonErrorState";
import type { FetchJsonLoadingState } from "./FetchJsonLoadingState";
import type { FetchJsonSuccessState } from "./FetchJsonSuccessState";

export type FetchJsonState<TResult> =
  | FetchJsonEmptyState
  | FetchJsonErrorState
  | FetchJsonLoadingState
  | FetchJsonSuccessState<TResult>;
