import type { z } from "zod";

import { HttpError } from "./HttpError";

export async function fetchJson<TSchema extends z.ZodType>({
  url,
  signal,
  schema,
}: {
  url: URL | string;
  signal: AbortSignal;
  schema: TSchema;
}): Promise<z.infer<TSchema>> {
  const response = await fetch(url, { signal });

  if (!response.ok) {
    throw new HttpError(
      response.status,
      `HTTP ${response.status} ${response.statusText}`,
    );
  }

  const payload: unknown = await response.json();

  return schema.parse(payload);
}
