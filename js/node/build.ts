import { NodeGlobalsPolyfillPlugin } from "@esbuild-plugins/node-globals-polyfill";
import { NodeModulesPolyfillPlugin } from "@esbuild-plugins/node-modules-polyfill";
import * as esbuild from "esbuild";

const buildOptions: esbuild.BuildOptions = {
  entryPoints: ["main.ts"],
  bundle: true,
  format: "iife",
  outfile: "dist/main.js",
  platform: "browser",
  target: "chrome58",
  plugins: [
    NodeModulesPolyfillPlugin(),
    NodeGlobalsPolyfillPlugin({
      buffer: true,
      process: true,
    }),
  ],
  define: {
    "process.env.NODE_ENV": '"production"',
  },
};

(async () => {
  await esbuild.build(buildOptions);
  console.log("Build complete");
})();
