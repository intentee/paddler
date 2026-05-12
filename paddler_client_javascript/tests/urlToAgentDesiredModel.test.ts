import { deepStrictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { urlToAgentDesiredModel } from "../src/urlToAgentDesiredModel";

test("recognizes Hugging Face URLs as HuggingFace variant", function () {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf",
  );

  deepStrictEqual(urlToAgentDesiredModel(url), {
    HuggingFace: {
      filename: "Qwen3-0.6B-Q8_0.gguf",
      repo_id: "Qwen/Qwen3-0.6B-GGUF",
      revision: "main",
    },
  });
});

test("agent: URLs become LocalToAgent variant", function () {
  const url = new URL("agent:///home/user/models/Qwen3-0.6B-Q8_0.gguf");

  deepStrictEqual(urlToAgentDesiredModel(url), {
    LocalToAgent: "/home/user/models/Qwen3-0.6B-Q8_0.gguf",
  });
});

test("unsupported URLs throw", function () {
  const url = new URL("https://example.com/some/path");

  throws(
    function () {
      urlToAgentDesiredModel(url);
    },
    { message: "Unsupported URL format" },
  );
});
