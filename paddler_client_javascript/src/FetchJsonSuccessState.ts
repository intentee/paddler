export type FetchJsonSuccessState<TResult> = {
  empty: false;
  error: null;
  loading: false;
  ok: true;
  response: TResult;
};
