#!/usr/bin/env -S deno run --allow-env --allow-read --allow-write --allow-net=deno.land
import $ from "dax";
import {
  CargoToml,
  processPlugin,
} from "https://raw.githubusercontent.com/dprint/automation/0.10.0/mod.ts";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const cargoFilePath = rootDir.join("plugin/Cargo.toml");

const pluginName = "dprint-plugin-svgo";
const version = new CargoToml(cargoFilePath).version();
const isTest = Deno.args.some((a) => a === "--test");
const githubOwner = "kjanat";
const githubRepo = pluginName;

const platforms = [
  "darwin-x86_64",
  "darwin-aarch64",
  "linux-x86_64",
  "linux-aarch64",
  "windows-x86_64",
] as const;

const builder = new processPlugin.PluginFileBuilder({
  name: pluginName,
  version,
});

for (const platform of platforms) {
  if (isTest && platform !== processPlugin.getCurrentPlatform()) {
    continue;
  }
  const zipFileName = processPlugin.getStandardZipFileName(
    pluginName,
    platform,
  );
  const zipFilePath = isTest
    ? rootDir.join("target/release").join(zipFileName).toString()
    : zipFileName;
  const zipUrl = isTest
    ? undefined
    : `https://github.com/${githubOwner}/${githubRepo}/releases/download/${version}/${zipFileName}`;
  await builder.addPlatform({
    platform,
    zipFilePath,
    zipUrl,
  });
}

await builder.writeToPath("plugin.json");
