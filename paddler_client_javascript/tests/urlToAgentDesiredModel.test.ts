import test from "ava";

import { urlToAgentDesiredModel } from "../src/urlToAgentDesiredModel";

test("recognizes Hugging Face URLs as HuggingFace variant", function (t) {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf",
  );

  t.deepEqual(urlToAgentDesiredModel(url), {
    HuggingFace: {
      filename: "Qwen3-0.6B-Q8_0.gguf",
      repo_id: "Qwen/Qwen3-0.6B-GGUF",
      revision: "main",
    },
  });
});

test("agent: URLs become LocalToAgent variant", function (t) {
  const url = new URL("agent:///home/user/models/Qwen3-0.6B-Q8_0.gguf");

  t.deepEqual(urlToAgentDesiredModel(url), {
    LocalToAgent: "/home/user/models/Qwen3-0.6B-Q8_0.gguf",
  });
});

test("unsupported URLs throw", function (t) {
  const url = new URL("https://example.com/some/path");

  t.throws(
    function () {
      urlToAgentDesiredModel(url);
    },
    { message: "Unsupported URL format" },
  );
});
