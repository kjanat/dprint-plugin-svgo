#!/usr/bin/env -S deno run -A

import { join } from "@std/path";
import { parse as parseToml } from "@std/toml";
import * as TJS from "npm:typescript-json-schema";
import { getSvgoDir, getSvgoVersion, rootDirPath } from "./lib.ts";

const schemaTypeName = "DprintPluginSvgoConfig";
const schemaTypesPath = rootDirPath.join("scripts", "schema_types.ts").toString();

export async function generateSchema(outputPath?: string) {
  const svgoDir = await getSvgoDir();
  const svgoTypesPath = join(svgoDir, "types", "lib", "svgo.d.ts");

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
      paths: {
        "svgo/browser": [svgoTypesPath],
      },
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

  const json = JSON.stringify(schema, null, 2) + "\n";

  if (outputPath) {
    await Deno.writeTextFile(outputPath, json);
    console.log(`\u2713 Wrote ${outputPath}`);
  } else {
    console.log(json);
  }

  console.error(`  Generated from scripts/schema_types.ts against SVGO ${svgoVersion}`);
}

if (import.meta.main) {
  await generateSchema(Deno.args[0]);
}
