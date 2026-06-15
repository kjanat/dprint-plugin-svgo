#!/usr/bin/env -S deno run --allow-read --frozen

import { getChecksum } from "dprint/automation/hash.ts";

const pluginPath = Deno.args[0] ?? "./target/release/plugin.json";

try {
  await Deno.stat(pluginPath);
} catch (error) {
  if (error instanceof Deno.errors.NotFound) {
    console.error(`Plugin file not found: ${pluginPath}`);
    Deno.exit(1);
  }

  console.error(`Failed to stat plugin file: ${pluginPath}`);
  console.error(error);
  Deno.exit(1);
}

const bytes = await Deno.readFile(pluginPath);
const checksum = await getChecksum(bytes);

console.log(`${pluginPath}@${checksum}`);
