#!/usr/bin/env -S deno run --allow-read --frozen

import { getChecksum } from "dprint/automation/hash.ts";

const pluginPath = Deno.args[0] ?? "./target/release/plugin.json";

await Deno.stat(pluginPath);

const bytes = await Deno.readFile(pluginPath);
const checksum = await getChecksum(bytes);

console.log(`${pluginPath}@${checksum}`);
