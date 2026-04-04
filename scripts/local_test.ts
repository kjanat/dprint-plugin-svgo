#!/usr/bin/env -S deno run -A

import { createLocalTestWorkspace, prepareLocalTestArtifacts, runDprintFormat } from "./lib.ts";

const { checksum, pluginFilePath } = await prepareLocalTestArtifacts();
const { configPath, tempDirPath } = await createLocalTestWorkspace(
  `${pluginFilePath}@${checksum}`,
);

try {
  await runDprintFormat(configPath);
} finally {
  await Deno.remove(tempDirPath, { recursive: true });
}
