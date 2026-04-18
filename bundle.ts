#!/usr/bin/env -S deno run -A --frozen

import { cli, command, flag } from "@kjanat/dreamcli";
import { parse } from "@std/jsonc";
import { build } from "tsdown";

type BundleName = "svgo-browser" | "svgo-runtime";
const bundleNames = ["svgo-browser", "svgo-runtime"] as const satisfies readonly BundleName[];

async function getRelativeImportAliases() {
  const denoConfigText = await Deno.readTextFile(new URL("./deno.jsonc", import.meta.url));
  const denoConfig = parse(denoConfigText) as {
    imports?: Record<string, unknown>;
  };
  const imports = denoConfig.imports ?? {};
  const aliases: Record<string, string> = {};

  for (const [name, value] of Object.entries(imports)) {
    if (typeof value === "string" && value.startsWith("./")) {
      aliases[name] = new URL(value, import.meta.url).href;
    }
  }

  return aliases;
}

async function buildBrowserBundle() {
  await build({
    config: false,
    name: "svgo-browser",
    entry: ["vendor/svgo/lib/svgo.js"],
    format: "esm",
    minify: true,
    platform: "browser",
    target: "esnext",
    outDir: "dist",
    clean: ["dist/svgo.mjs"],
    dts: false,
  });
}

async function buildRuntimeBundle() {
  const alias = await getRelativeImportAliases();
  await build({
    config: false,
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
  });
}

await cli({ name: "bundle", inherit: true })
  .default(
    command("build")
      .flag(
        "filter",
        flag.enum(bundleNames).env("FILTER").describe("Only build the selected bundle"),
      )
      .action(async ({ flags }) => {
        if (flags.filter === undefined || flags.filter === "svgo-browser") {
          await buildBrowserBundle();
        }

        if (flags.filter === undefined || flags.filter === "svgo-runtime") {
          await buildRuntimeBundle();
        }
      }),
  )
  .run();
