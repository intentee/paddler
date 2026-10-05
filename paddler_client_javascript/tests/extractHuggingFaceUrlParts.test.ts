import { deepStrictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { extractHuggingFaceUrlParts } from "../src/extractHuggingFaceUrlParts";

test("blob URL extracts owner, repo, revision and filename", function () {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf",
  );

  deepStrictEqual(extractHuggingFaceUrlParts(url), {
    filename: "Qwen3-0.6B-Q8_0.gguf",
    repo_id: "Qwen/Qwen3-0.6B-GGUF",
    revision: "main",
  });
});

test("resolve URL extracts the same fields", function () {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf",
  );

  deepStrictEqual(extractHuggingFaceUrlParts(url), {
    filename: "Qwen3-0.6B-Q8_0.gguf",
    repo_id: "Qwen/Qwen3-0.6B-GGUF",
    revision: "main",
  });
});

test("nested filename paths preserve every segment", function () {
  const url = new URL(
    "https://huggingface.co/owner/repo/blob/main/dir/sub/file.gguf",
  );

  deepStrictEqual(extractHuggingFaceUrlParts(url), {
    filename: "dir/sub/file.gguf",
    repo_id: "owner/repo",
    revision: "main",
  });
});

test("malformed URLs throw", function () {
  const url = new URL("https://huggingface.co/owner/repo");

  throws(function () {
    extractHuggingFaceUrlParts(url);
  });
});
