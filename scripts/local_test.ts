#!/usr/bin/env -S deno run -A

import $ from "dax";
import { getChecksum } from "dprint/automation/hash.ts";

await $`./scripts/create_for_testing.ts`;
const bytes = $.path("./target/release/plugin.json").readBytesSync();
const checksum = await getChecksum(
  new Uint8Array(bytes.slice().buffer),
);
const dprintConfig = $.path("dprint.json");
const data = dprintConfig.readJsonSync<{ plugins: string[] }>();
const index = data.plugins.findIndex((d) => d.startsWith("./target") || d.includes("svgo"));
data.plugins[index] = `./target/release/plugin.json@${checksum}`;
dprintConfig.writeJsonPrettySync(data);
await $`dprint fmt`;
