#!/usr/bin/env -S deno run -A
import { createPluginFile } from "./lib.ts";

if (import.meta.main) {
  await createPluginFile({
    outputDir: Deno.cwd(),
    test: Deno.args.includes("--test"),
  });
}
