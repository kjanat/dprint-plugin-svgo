#!/usr/bin/env -S deno run -A

/**
 * This script checks for any svgo updates and then automatically
 * publishes a new version of the plugin if so.
 */
import { parse as parseToml } from "@std/toml";
import * as semver from "semver";
import $ from "dax";

const rootDirPath = $.path(import.meta.dirname!).parentOrThrow();

$.logStep("Upgrading svgo...");
await $`deno add npm:svgo`.cwd(rootDirPath);

if (!await hasFileChanged("./deno.jsonc") && !await hasFileChanged("./deno.lock")) {
  $.log("No changes.");
  Deno.exit(0);
}

$.log("Found changes.");

$.logStep("Rebuilding...");
await $`deno task build`.cwd(rootDirPath);
await $`deno task schema`.cwd(rootDirPath);

$.logStep("Bumping version...");
const newVersion = await bumpMinorVersion();

$.logStep("Running tests...");
await $`cargo test`;

$.logStep(`Committing and publishing ${newVersion}...`);
await $`git add deno.jsonc deno.lock Cargo.toml Cargo.lock schema.json`;
await $`git commit -m ${newVersion}`;
await $`git push origin master`;
await $`git tag ${newVersion}`;
await $`git push origin ${newVersion}`;

async function bumpMinorVersion() {
  const projectFile = rootDirPath.join("./Cargo.toml");
  const text = await projectFile.readText();
  const cargo = parseToml(text) as {
    workspace: { package: { version: string } };
  };
  const currentVersion = cargo.workspace.package.version;
  const newVersion = semver.format(
    semver.increment(semver.parse(currentVersion), "minor"),
  );
  const oldLiteral = `"${currentVersion}"`;
  const newLiteral = `"${newVersion}"`;
  if (!text.includes(oldLiteral)) {
    throw new Error(`Version literal ${oldLiteral} not found in Cargo.toml`);
  }
  const newText = text.replace(oldLiteral, newLiteral);
  await projectFile.writeText(newText);
  return newVersion;
}

async function hasFileChanged(file: string) {
  try {
    await $`git diff --exit-code ${file}`;
    return false;
  } catch {
    return true;
  }
}
