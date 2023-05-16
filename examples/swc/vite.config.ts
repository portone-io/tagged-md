import { createRequire } from "node:module";
import { defineConfig } from "vite";
import { swc, defineRollupSwcOption } from "rollup-plugin-swc3";

const require = createRequire(import.meta.url);

export default defineConfig({
  plugins: [
    swc(
      defineRollupSwcOption({
        jsc: {
          experimental: {
            plugins: [[require.resolve("swc-plugin-tagged-md"), { gfm: true }]],
          },
        },
      })
    ),
  ],
});
