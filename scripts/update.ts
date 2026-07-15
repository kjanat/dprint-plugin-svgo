#!/usr/bin/env -S deno run -A

/**
 * This script checks for any svgo updates and then automatically
 * publishes a new version of the plugin if so.
 */
import { parse as parseToml } from "@std/toml";
import * as semver from "semver";
import $ from "dax";
import {
  buildJsBundle,
  cargoTestAllFeatures,
  getSvgoVersion,
  refreshDenoLock,
  rootDirPath,
  syncSvgoDenoImports,
  vendorSvgoDirPath,
} from "./lib.ts";

$.logStep("Fetching svgo tags...");
await $`git fetch --tags origin`.cwd(vendorSvgoDirPath);

// Derive the vendored version from package.json rather than requiring the
// submodule HEAD to land exactly on a tag: SVGO's version-bump commit is not
// the commit that carries the release tag, so a vendored pin can legitimately
// sit one commit away from its `v*` tag.
const currentVersion = await getSvgoVersion();
const latestTag = await getLatestSvgoTag();
const latestVersion = latestTag.replace(/^v/, "");

if (isUpToDate(currentVersion, latestVersion)) {
  $.log(`SVGO already at ${currentVersion}.`);
  Deno.exit(0);
}

$.logStep(`Upgrading svgo from ${currentVersion} to ${latestVersion}...`);
await $`git checkout ${latestTag}`.cwd(vendorSvgoDirPath);

$.logStep("Syncing Deno imports and lockfile...");
await syncSvgoDenoImports();
await refreshDenoLock();

$.logStep("Bumping version...");
const newVersion = await bumpMinorVersion();

$.logStep("Rebuilding...");
await buildJsBundle();

$.logStep("Running tests...");
await cargoTestAllFeatures();

$.logStep(`Committing and publishing ${newVersion}...`);
await $`git add -f .gitmodules vendor/svgo deno.jsonc deno.lock Cargo.toml Cargo.lock`;
await $`git commit -m ${newVersion}`;
await $`git push origin HEAD:master`;
await $`git tag ${newVersion}`;
await $`git push origin ${newVersion}`;

async function bumpMinorVersion() {
  const projectFile = rootDirPath.join("Cargo.toml");
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

function isUpToDate(currentVersion: string, latestVersion: string) {
  const current = semver.parse(currentVersion);
  const latest = semver.parse(latestVersion);
  if (current == null || latest == null) {
    throw new Error(
      `Expected valid semver versions, got ${currentVersion} and ${latestVersion}.`,
    );
  }
  return semver.greaterOrEqual(current, latest);
}

async function getLatestSvgoTag() {
  const lines = await $`git tag --list v*`.cwd(vendorSvgoDirPath).text();
  const tags = lines
    .split(/\r?\n/)
    .map((line: string) => line.trim())
    .filter((line: string) => /^v\d/.test(line));
  if (tags.length === 0) {
    throw new Error("Could not find any SVGO version tags.");
  }
  return sortSemverTags(tags).at(-1)!;
}

function sortSemverTags(tags: string[]) {
  return tags.toSorted((a, b) => {
    const left = semver.parse(a.replace(/^v/, ""));
    const right = semver.parse(b.replace(/^v/, ""));
    if (left == null || right == null) {
      throw new Error(`Expected valid semver tags, got ${a} and ${b}.`);
    }
    return semver.compare(left, right);
  });
}
