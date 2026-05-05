export type FetchJsonLoadingState = {
  empty: false;
  error: null;
  loading: true;
  ok: false;
  response: null;
};

export const fetchJsonLoadingState: FetchJsonLoadingState = Object.freeze({
  empty: false,
  error: null,
  loading: true,
  ok: false,
  response: null,
});
