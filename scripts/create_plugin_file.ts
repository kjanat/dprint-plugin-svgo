#!/usr/bin/env -S deno run -A
import {
  CargoToml,
  processPlugin,
} from "https://raw.githubusercontent.com/dprint/automation/0.10.3/mod.ts";
import $ from "dax";

const GITHUB_OWNER = "kjanat";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const cargoFilePath = rootDir.join("plugin/Cargo.toml");
const pluginName = "dprint-plugin-svgo";
const version = new CargoToml(cargoFilePath).version();
const isTest = Deno.args.some((a) => a === "--test");

const platforms: processPlugin.Platform[] = [
  "darwin-x86_64",
  "darwin-aarch64",
  "linux-x86_64",
  "linux-aarch64",
  "windows-x86_64",
];

const builder = new processPlugin.PluginFileBuilder({
  name: pluginName,
  version,
});

if (isTest) {
  const platform = processPlugin.getCurrentPlatform();
  const zipFileName = processPlugin.getStandardZipFileName(pluginName, platform);
  await builder.addPlatform({
    platform,
    zipFilePath: zipFileName,
    zipUrl: zipFileName,
  });
} else {
  for (const platform of platforms) {
    const zipFileName = processPlugin.getStandardZipFileName(pluginName, platform);
    const zipUrl = `https://github.com/${GITHUB_OWNER}/${pluginName}/releases/download/${version}/${zipFileName}`;
    await builder.addPlatform({
      platform,
      zipFilePath: zipFileName,
      zipUrl,
    });
  }
}

await builder.writeToPath("plugin.json");
