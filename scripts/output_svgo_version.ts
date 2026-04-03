#!/usr/bin/env -S deno run -A

import { dirname, join } from "@std/path";

const svgoBrowserUrl = import.meta.resolve("npm:svgo/browser");
const svgoDir = dirname(dirname(new URL(svgoBrowserUrl).pathname));
const svgoPkg = JSON.parse(
  await Deno.readTextFile(join(svgoDir, "package.json")),
);
console.log(svgoPkg.version);
