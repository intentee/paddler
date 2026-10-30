import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

const eslintConfig = {
  ...eslint.configs.recommended,
};

eslintConfig.rules = {
  ...eslintConfig.rules,

  "func-style": ["error", "declaration"],
  "prefer-template": "error",
  "require-await": "error",
};

export default tseslint.config(
  {
    extends: [
      eslintConfig,
      tseslint.configs.strictTypeChecked,
      reactHooks.configs.flat.recommended,
    ],
    files: ["resources/ts/**/*.{ts,tsx}"],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      "@typescript-eslint/ban-ts-comment": [
        "error",
        { "ts-ignore": "allow-with-description" },
      ],
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-invalid-void-type": [
        "error",
        { allowAsThisParameter: true },
      ],
      "@typescript-eslint/no-unsafe-assignment": "off",
      "@typescript-eslint/restrict-template-expressions": [
        "error",
        { allowNumber: true },
      ],
    },
  },
  {
    extends: [tseslint.configs.disableTypeChecked],
    files: ["resources/ts/polyfill_*.ts"],
  },
  {
    extends: [eslintConfig],
    files: ["jarmuz/**/*.mjs"],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
);

