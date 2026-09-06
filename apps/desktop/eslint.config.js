import tseslint from "@typescript-eslint/eslint-plugin";
import parser from "@typescript-eslint/parser";

export default [{ files: ["src/**/*.{ts,tsx}"], languageOptions: { parser, parserOptions: { ecmaVersion: "latest", sourceType: "module", ecmaFeatures: { jsx: true } } }, plugins: { "@typescript-eslint": tseslint }, rules: { ...tseslint.configs.recommended.rules } }];
