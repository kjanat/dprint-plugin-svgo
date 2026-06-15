#!/usr/bin/env -S deno run -A

import { createLocalTestWorkspace, prepareLocalTestArtifacts, runDprintFormat } from "./lib.ts";

const timeoutSeconds = getLocalTestTimeoutSeconds();

console.error("Preparing local test artifacts...");
const { checksum } = await prepareLocalTestArtifacts();
console.error("Creating disposable local-test workspace...");
const { configPath, tempDirPath } = await createLocalTestWorkspace(checksum);

try {
  console.error(`Running dprint format smoke test (timeout: ${timeoutSeconds}s)...`);
  await runDprintFormat(configPath, "fixture.svg", timeoutSeconds * 1000);
} finally {
  console.error("Cleaning up local-test workspace...");
  await Deno.remove(tempDirPath, { recursive: true });
}

function getLocalTestTimeoutSeconds() {
  const value = Deno.env.get("LOCAL_TEST_TIMEOUT_SECONDS");
  if (value == null) {
    return 20;
  }

  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`LOCAL_TEST_TIMEOUT_SECONDS must be a positive integer, got: ${value}`);
  }

  if (parsed > 60) {
    throw new Error("LOCAL_TEST_TIMEOUT_SECONDS must be <= 60.");
  }

  return parsed;
}
