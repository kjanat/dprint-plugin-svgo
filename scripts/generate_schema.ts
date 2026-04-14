#!/usr/bin/env -S deno run -A

import { parse as parseToml } from "@std/toml";
import * as TJS from "npm:typescript-json-schema";
import { getSvgoCompilerPaths, getSvgoVersion, rootDirPath } from "./lib.ts";

const schemaTypeName = "DprintPluginSvgoConfig";
const schemaTypesPath = rootDirPath.join("scripts", "schema_types.ts").toString();
const svgoPluginsDirPath = rootDirPath.join("vendor", "svgo", "plugins");

interface SchemaMeta {
  pluginVersion: string;
  svgoVersion: string;
  pluginNames: string[];
  presetDefault: string[];
  pluginDescriptions: Record<string, string>;
}

interface PluginMetadata {
  name: string;
  description: string;
}

type ExportName = "name" | "description";

const EXPORT_PATTERNS: Record<ExportName, RegExp> = {
  name: /export const name =\s*(['"])([\s\S]*?)\1\s*;/,
  description: /export const description =\s*(['"])([\s\S]*?)\1\s*;/,
};

export async function generateSchema(outputPath?: string) {
  const cargo = parseToml(await Deno.readTextFile(rootDirPath.join("Cargo.toml").toString())) as {
    workspace: { package: { version: string } };
  };
  const pluginVersion = cargo.workspace.package.version;
  const svgoVersion = await getSvgoVersion();

  const program = TJS.getProgramFromFiles(
    [schemaTypesPath],
    {
      allowJs: true,
      baseUrl: rootDirPath.toString(),
      checkJs: false,
      module: "ESNext",
      moduleResolution: "Node",
      paths: await getSvgoCompilerPaths(),
      strictNullChecks: true,
      target: "ES2022",
    },
    rootDirPath.toString(),
  );

  const schema = TJS.generateSchema(program, schemaTypeName, {
    id: `https://plugins.dprint.dev/kjanat/dprint-plugin-svgo/${pluginVersion}/schema.json`,
    ignoreErrors: false,
    noExtraProps: true,
    required: true,
    titles: false,
  });

  if (schema == null) {
    throw new Error(`Failed to generate schema for ${schemaTypeName}.`);
  }

  const schemaWithMeta = {
    ...schema,
    _meta: await getSchemaMeta(pluginVersion, svgoVersion),
  };
  const json = JSON.stringify(schemaWithMeta, null, 2) + "\n";

  if (outputPath) {
    await Deno.writeTextFile(outputPath, json);
    console.log(`\u2713 Wrote ${outputPath}`);
  } else {
    console.log(json);
  }

  console.error(`  Generated from scripts/schema_types.ts against SVGO ${svgoVersion}`);
}

async function getSchemaMeta(pluginVersion: string, svgoVersion: string): Promise<SchemaMeta> {
  const plugins = await readBuiltinPluginMetadata();
  const presetDefault = await readPresetDefaultPluginNames();
  const defaultPluginNames = new Set(presetDefault);
  const extraPluginNames = plugins.map((plugin) => plugin.name).filter((name) =>
    !defaultPluginNames.has(name)
  );

  return {
    pluginVersion,
    svgoVersion,
    pluginNames: ["preset-default", ...presetDefault, ...extraPluginNames],
    presetDefault,
    pluginDescriptions: Object.fromEntries(
      plugins.map((plugin) => [plugin.name, plugin.description]),
    ),
  };
}

async function readBuiltinPluginMetadata(): Promise<PluginMetadata[]> {
  const pluginFiles: string[] = [];
  for await (const entry of Deno.readDir(svgoPluginsDirPath.toString())) {
    if (!entry.isFile || !entry.name.endsWith(".js")) continue;
    if (entry.name.startsWith("_") || entry.name === "preset-default.js") continue;
    pluginFiles.push(entry.name);
  }
  pluginFiles.sort((left, right) => left.localeCompare(right));

  const plugins = await Promise.all(pluginFiles.map(async (fileName) => {
    const source = await Deno.readTextFile(svgoPluginsDirPath.join(fileName).toString());
    const name = tryParseStringExport(source, "name");
    if (name == null) {
      return null;
    }

    const description = tryParseStringExport(source, "description");
    if (description == null) {
      throw new Error(`Could not find exported string description in ${fileName}.`);
    }

    return {
      name,
      description: normalizeStringValue(description),
    };
  }));

  return plugins.filter((plugin): plugin is PluginMetadata => plugin != null);
}

async function readPresetDefaultPluginNames(): Promise<string[]> {
  const source = await Deno.readTextFile(svgoPluginsDirPath.join("preset-default.js").toString());
  const match = source.match(/plugins:\s*\[([\s\S]*?)\]/);
  if (match == null) {
    throw new Error("Could not find preset-default plugin list.");
  }

  return [...match[1].matchAll(/^\s*([A-Za-z0-9_]+),?$/gm)].map((pluginMatch) => pluginMatch[1]);
}

function tryParseStringExport(source: string, exportName: ExportName): string | undefined {
  const match = source.match(EXPORT_PATTERNS[exportName]);
  return match?.[2];
}

function normalizeStringValue(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

if (import.meta.main) {
  await generateSchema(Deno.args[0]);
}
