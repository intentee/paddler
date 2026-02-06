#!/usr/bin/env node

import { jarmuz } from "jarmuz";

jarmuz({
  once: true,
  pipeline: ["cargo-fmt", "prettier"],
  watch: [
    "jarmuz",
    "paddler",
    "paddler_client",
    "paddler_types",
    "resources",
    "templates",
    "*.mjs",
  ],
}).decide(function ({ matches, schedule }) {
  switch (true) {
    case matches("**/*.css"):
    case matches("**/*.mjs"):
    case matches("**/*.ts"):
    case matches("**/*.tsx"):
      schedule("prettier");
      break;
    case matches("**/*.rs"):
      schedule("cargo-fmt");
      break;
  }
});
