import { z } from "zod";

export const InferenceServiceGenerateTokensResponseSchema = z
  .union([
    z.object({
      Error: z.object({
        error: z.object({
          code: z.number(),
          description: z.string(),
        }),
        request_id: z.string(),
      }),
    }),
    z.object({
      Response: z.object({
        request_id: z.string(),
        response: z.object({
          GeneratedToken: z.union([
            z.object({
              ChatTemplateError: z.string(),
            }),
            z.literal("Done"),
            z.object({
              ImageDecodingFailed: z.string(),
            }),
            z.object({
              MultimodalNotSupported: z.string(),
            }),
            z.object({
              ThinkingToken: z.string(),
            }),
            z.object({
              Token: z.string(),
            }),
          ]),
        }),
      }),
    }),
  ])
  .transform(function (data):
    | {
        done: true;
        error: null;
        ok: true;
        request_id: string;
        thinking_token: null;
        token: null;
      }
    | {
        done: false;
        error: null;
        ok: true;
        request_id: string;
        thinking_token: null;
        token: string;
      }
    | {
        done: false;
        error: null;
        ok: true;
        request_id: string;
        thinking_token: string;
        token: null;
      }
    | {
        done: true;
        error: {
          code: number;
          description: string;
        };
        ok: false;
        request_id: string;
        thinking_token: null;
        token: null;
      } {
    if ("Error" in data) {
      return Object.freeze({
        done: true,
        error: data.Error.error,
        ok: false,
        request_id: data.Error.request_id,
        thinking_token: null,
        token: null,
      });
    }

    if (data.Response.response.GeneratedToken === "Done") {
      return Object.freeze({
        done: true,
        error: null,
        ok: true,
        request_id: data.Response.request_id,
        thinking_token: null,
        token: null,
      });
    }

    if ("ChatTemplateError" in data.Response.response.GeneratedToken) {
      return Object.freeze({
        done: true,
        error: Object.freeze({
          code: 500,
          description: data.Response.response.GeneratedToken.ChatTemplateError,
        }),
        ok: false,
        request_id: data.Response.request_id,
        thinking_token: null,
        token: null,
      });
    }

    if ("ImageDecodingFailed" in data.Response.response.GeneratedToken) {
      return Object.freeze({
        done: true,
        error: Object.freeze({
          code: 400,
          description:
            data.Response.response.GeneratedToken.ImageDecodingFailed,
        }),
        ok: false,
        request_id: data.Response.request_id,
        thinking_token: null,
        token: null,
      });
    }

    if ("MultimodalNotSupported" in data.Response.response.GeneratedToken) {
      return Object.freeze({
        done: true,
        error: Object.freeze({
          code: 400,
          description:
            data.Response.response.GeneratedToken.MultimodalNotSupported,
        }),
        ok: false,
        request_id: data.Response.request_id,
        thinking_token: null,
        token: null,
      });
    }

    if ("ThinkingToken" in data.Response.response.GeneratedToken) {
      return Object.freeze({
        done: false,
        error: null,
        ok: true,
        request_id: data.Response.request_id,
        thinking_token: data.Response.response.GeneratedToken.ThinkingToken,
        token: null,
      });
    }

    if ("Token" in data.Response.response.GeneratedToken) {
      return Object.freeze({
        done: false,
        error: null,
        ok: true,
        request_id: data.Response.request_id,
        thinking_token: null,
        token: data.Response.response.GeneratedToken.Token,
      });
    }

    return Object.freeze({
      done: true,
      error: null,
      ok: true,
      request_id: data.Response.request_id,
      thinking_token: null,
      token: null,
    });
  });

export type InferenceServiceGenerateTokensResponse = z.infer<
  typeof InferenceServiceGenerateTokensResponseSchema
>;
