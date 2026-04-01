#!/usr/bin/env -S deno run --allow-env --allow-read --allow-write --allow-net=deno.land
import $ from "dax";
import {
  CargoToml,
  getChecksum,
} from "https://raw.githubusercontent.com/dprint/automation/0.10.0/mod.ts";

const GITHUB_OWNER = "kjanat";
const PLUGIN_NAME = "dprint-plugin-svgo";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const cargoFilePath = rootDir.join("plugin/Cargo.toml");
const version = new CargoToml(cargoFilePath).version();
const isTest = Deno.args.some((a) => a === "--test");

const platformMap: Record<string, string> = {
  "darwin-x86_64": "x86_64-apple-darwin",
  "darwin-aarch64": "aarch64-apple-darwin",
  "linux-x86_64": "x86_64-unknown-linux-gnu",
  "linux-aarch64": "aarch64-unknown-linux-gnu",
  "windows-x86_64": "x86_64-pc-windows-msvc",
};

const platforms = [
  "darwin-x86_64",
  "darwin-aarch64",
  "linux-x86_64",
  "linux-aarch64",
  "windows-x86_64",
];

// deno-lint-ignore no-explicit-any
const plugin: Record<string, any> = {
  schemaVersion: 2,
  kind: "process",
  name: PLUGIN_NAME,
  version,
};

for (const platform of platforms) {
  const triple = platformMap[platform];
  const zipFileName = `${PLUGIN_NAME}-${triple}.zip`;

  if (isTest) {
    // In test mode, use only the current platform with a local reference
    const currentPlatform = `${Deno.build.os === "darwin" ? "darwin" : Deno.build.os === "windows" ? "windows" : "linux"}-${Deno.build.arch === "aarch64" ? "aarch64" : "x86_64"}`;
    if (platform !== currentPlatform) continue;
    const zipPath = rootDir.join(`target/release/${zipFileName}`);
    const checksum = getChecksum(zipPath.readBytesSync());
    plugin[platform] = {
      reference: `./${zipFileName}`,
      checksum,
    };
  } else {
    const url = `https://github.com/${GITHUB_OWNER}/${PLUGIN_NAME}/releases/download/${version}/${zipFileName}`;
    const zipPath = rootDir.join(zipFileName);
    const checksum = getChecksum(zipPath.readBytesSync());
    plugin[platform] = {
      reference: url,
      checksum,
    };
  }
}

Deno.writeTextFileSync("plugin.json", JSON.stringify(plugin, null, 2) + "\n");
