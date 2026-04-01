#!/usr/bin/env -S deno run -A

import $ from "dax";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const packageJson = rootDir.join("js/node/package.json").readJsonSync<
  { dependencies: Record<string, string> }
>();
console.log(packageJson.dependencies["svgo"].replace("^", ""));
