// <app> — hexagonal discipline, ESLint flat config. This is the depguard
// analog from the Go template, mapped onto Node's native package subpath
// imports (see package.json "imports"):
//
//   #shared/*, #platform/*, #features/*, #gen/*  → subpath imports
//   own-slice files                              → relative imports ('./x')
//
// Rules (verbatim from .golangci.yml's two depguard rule groups):
//   domain-pure:     features/*/domain.ts imports no transport, no persistence,
//                    no platform infra, no generated code, no other slice.
//   slice-isolation: the features tree never imports another slice via
//                    #features/* — route composition lives in bin/api.ts.
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/", "node_modules/", "coverage/", "src/gen/"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      // `_`-prefixed unused args (interface-shape alignment, deliberate
      // disuses) are a convention, not slop.
      "@typescript-eslint/no-unused-vars": [
        "error",
        { args: "all", argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
  {
    // slice-isolation — the features tree is sealed across slices.
    files: ["src/features/**"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["#features/*"],
              message:
                "vertical slices are sealed — route composition through bin/api.ts; import within your own slice relatively",
            },
          ],
        },
      ],
    },
  },
  {
    // domain-pure — domain.ts on top of slice-isolation (configs merge here:
    // this is the most specific match, so it carries the union).
    files: ["src/features/**/domain.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["#features/*"],
              message: "slices never depend on other slices",
            },
            {
              group: ["#platform/*"],
              message: "domain depends on its own ports, not platform infra",
            },
            {
              group: ["#gen/*"],
              message: "generated db/api code is an adapter concern, not domain",
            },
            {
              group: ["pg", "kysely"],
              message: "persistence is a port; use interfaces in ports.ts",
            },
            {
              group: ["fastify", "@fastify/*"],
              message: "domain stays transport-agnostic — adapters live in http.ts",
            },
          ],
        },
      ],
    },
  },
);
