#!/usr/bin/env -S deno run -A

import { getSvgoVersion } from "./lib.ts";

if (import.meta.main) {
  console.log(await getSvgoVersion());
}
