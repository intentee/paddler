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

test("non-http(s), non-agent URLs throw", function () {
  const url = new URL("ftp://example.com/file.gguf");

  throws(
    function () {
      urlToAgentDesiredModel(url);
    },
    { message: "Unsupported URL format" },
  );
});

test("the user's Qwen 3.6 35B blob URL still routes to HuggingFace", function () {
  const url = new URL(
    "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF/blob/main/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
  );

  deepStrictEqual(urlToAgentDesiredModel(url), {
    HuggingFace: {
      filename: "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
      repo_id: "unsloth/Qwen3.6-35B-A3B-GGUF",
      revision: "main",
    },
  });
});

test("https URLs off huggingface.co route to the Url variant", function () {
  const url = new URL("https://example.com/path/to/model.gguf");

  deepStrictEqual(urlToAgentDesiredModel(url), {
    Url: { url: "https://example.com/path/to/model.gguf" },
  });
});

test("plain http URLs route to the Url variant", function () {
  const url = new URL("http://mirror.example.org/Qwen3-0.6B.gguf");

  deepStrictEqual(urlToAgentDesiredModel(url), {
    Url: { url: "http://mirror.example.org/Qwen3-0.6B.gguf" },
  });
});
