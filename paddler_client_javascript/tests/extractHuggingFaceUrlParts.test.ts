import test from "ava";

import { extractHuggingFaceUrlParts } from "../src/extractHuggingFaceUrlParts";

test("blob URL extracts owner, repo, revision and filename", function (t) {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q8_0.gguf",
  );

  t.deepEqual(extractHuggingFaceUrlParts(url), {
    filename: "Qwen3-0.6B-Q8_0.gguf",
    repo_id: "Qwen/Qwen3-0.6B-GGUF",
    revision: "main",
  });
});

test("resolve URL extracts the same fields", function (t) {
  const url = new URL(
    "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf",
  );

  t.deepEqual(extractHuggingFaceUrlParts(url), {
    filename: "Qwen3-0.6B-Q8_0.gguf",
    repo_id: "Qwen/Qwen3-0.6B-GGUF",
    revision: "main",
  });
});

test("nested filename paths preserve every segment", function (t) {
  const url = new URL(
    "https://huggingface.co/owner/repo/blob/main/dir/sub/file.gguf",
  );

  t.deepEqual(extractHuggingFaceUrlParts(url), {
    filename: "dir/sub/file.gguf",
    repo_id: "owner/repo",
    revision: "main",
  });
});

test("malformed URLs throw", function (t) {
  const url = new URL("https://huggingface.co/owner/repo");

  t.throws(function () {
    extractHuggingFaceUrlParts(url);
  });
});
