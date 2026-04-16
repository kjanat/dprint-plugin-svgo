import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defineConfig } from "tsdown";

function getRelativeImportAliases() {
  const denoConfigText = readFileSync(resolve(process.cwd(), "deno.jsonc"), "utf8");
  const denoConfig = JSON.parse(denoConfigText) as {
    imports?: Record<string, unknown>;
  };
  const imports = denoConfig.imports ?? {};
  const aliases: Record<string, string> = {};

  for (const [name, value] of Object.entries(imports)) {
    if (typeof value === "string" && value.startsWith("./")) {
      aliases[name] = resolve(process.cwd(), value);
    }
  }

  return aliases;
}

const alias = getRelativeImportAliases();

export default defineConfig([
  {
    name: "svgo-browser",
    entry: ["vendor/svgo/lib/svgo.js"],
    format: "esm",
    minify: true,
    platform: "browser",
    target: "esnext",
    outDir: "dist",
    clean: ["dist/svgo.mjs"],
    dts: false,
  },
  {
    name: "svgo-runtime",
    entry: ["js/svgo.ts"],
    format: "iife",
    minify: true,
    platform: "browser",
    target: "esnext",
    outDir: "js/dist",
    clean: ["js/dist/svgo.js"],
    outputOptions: {
      entryFileNames: "svgo.js",
    },
    alias,
    dts: false,
  },
]);
