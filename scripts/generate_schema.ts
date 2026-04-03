#!/usr/bin/env -S deno run -A
/**
 * Generate JSON Schema directly from SVGO's source files.
 *
 * Resolves SVGO via Deno's npm resolution, then statically parses plugin
 * files to extract names, descriptions, and preset-default membership.
 *
 * Run: deno task generate-schema [output-path]
 */

import { dirname, join } from "@std/path";
import { parse as parseToml } from "@std/toml";

const ROOT = new URL("..", import.meta.url).pathname;

// Deno resolves + downloads svgo on the fly — derive package root from resolved path
const svgoBrowserUrl = import.meta.resolve("npm:svgo@^4/browser");
const SVGO_DIR = dirname(dirname(new URL(svgoBrowserUrl).pathname));

// ---------------------------------------------------------------------------
// 1. Statically extract plugin info (same parsing as approach A)
// ---------------------------------------------------------------------------

interface PluginInfo {
  name: string;
  description: string;
}

function parsePluginFile(source: string): { name: string; description: string } {
  const nameMatch = source.match(/export\s+const\s+name\s*=\s*['"]([^'"]+)['"]/);
  const descMatch = source.match(/export\s+const\s+description\s*=\s*\n?\s*['"]([^'"]+)['"]/);
  return {
    name: nameMatch?.[1] ?? "",
    description: descMatch?.[1] ?? "",
  };
}

function parseBuiltinImports(source: string): string[] {
  const imports: string[] = [];
  for (const match of source.matchAll(/from\s+['"]\.\.\/plugins\/([^'"]+)\.js['"]/g)) {
    const filename = match[1];
    if (!filename.startsWith("_")) imports.push(filename);
  }
  return imports;
}

const builtinSource = await Deno.readTextFile(join(SVGO_DIR, "lib/builtin.js"));
const pluginFilenames = parseBuiltinImports(builtinSource);

const allPlugins: PluginInfo[] = [];
const presetNames: string[] = [];

for (const filename of pluginFilenames) {
  const source = await Deno.readTextFile(join(SVGO_DIR, "plugins", `${filename}.js`));
  if (filename.startsWith("preset-")) {
    presetNames.push(filename);
  } else {
    const info = parsePluginFile(source);
    if (info.name) allPlugins.push(info);
    else console.error(`\u26a0 Could not extract name from ${filename}.js`);
  }
}

// Parse preset-default membership
const presetDefaultSource = await Deno.readTextFile(
  join(SVGO_DIR, "plugins/preset-default.js"),
);
const defaultImportNames = new Set<string>();
for (
  const match of presetDefaultSource.matchAll(
    /import\s+\*\s+as\s+\w+\s+from\s+['"]\.\/([^'"]+)\.js['"]/g,
  )
) {
  defaultImportNames.add(match[1]);
}

// ---------------------------------------------------------------------------
// 2. Read versions
// ---------------------------------------------------------------------------

const cargo = parseToml(await Deno.readTextFile(join(ROOT, "Cargo.toml"))) as {
  workspace: { package: { version: string } };
};
const pluginVersion = cargo.workspace.package.version;

const svgoPackageJson = JSON.parse(
  await Deno.readTextFile(join(SVGO_DIR, "package.json")),
);
const svgoVersion: string = svgoPackageJson.version;

// ---------------------------------------------------------------------------
// 3. Build JSON Schema
// ---------------------------------------------------------------------------

const pluginNameEnum = [...presetNames, ...allPlugins.map((p) => p.name)];
const pluginDescriptions = Object.fromEntries(
  allPlugins.map((p) => [p.name, p.description]),
);

const pluginEntrySchema = {
  oneOf: [
    {
      type: "string" as const,
      enum: pluginNameEnum,
      description: "Plugin or preset referenced by name.",
    },
    {
      type: "object" as const,
      required: ["name"],
      properties: {
        name: {
          type: "string" as const,
          enum: pluginNameEnum,
          description: "Plugin or preset name.",
        },
        params: {
          description: "Plugin parameters. Structure depends on the plugin. " +
            "For preset-default, use `overrides` to disable/configure individual plugins.",
        },
      },
      additionalProperties: false,
    },
  ],
};

const eolEnum = { type: "string" as const, enum: ["lf", "crlf"] };

const js2svgSchema = {
  type: "object" as const,
  description: "Direct js2svg configuration object passed to SVGO. " +
    "Overrides top-level indent, eol, pretty, finalNewline, and useShortTags.",
  properties: {
    indent: {
      type: "integer" as const,
      minimum: 0,
      description: "Number of spaces for indentation.",
    },
    eol: { ...eolEnum, description: 'End-of-line character ("lf" or "crlf").' },
    pretty: {
      type: "boolean" as const,
      description: "Whether to pretty-print the output.",
    },
    finalNewline: {
      type: "boolean" as const,
      description: "Whether to add a final newline at the end of the output.",
    },
    useShortTags: {
      type: "boolean" as const,
      description:
        "Whether to use short self-closing tags (e.g. `<path/>` instead of `<path></path>`).",
    },
  },
  additionalProperties: false,
};

const schema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  $id: `https://plugins.dprint.dev/kjanat/dprint-plugin-svgo/${pluginVersion}/schema.json`,
  title: "dprint-plugin-svgo configuration",
  type: "object",
  properties: {
    indent: {
      type: "integer",
      minimum: 0,
      description:
        "Number of spaces for indentation in SVG output. Inherited from dprint global indentWidth when unset.",
    },
    eol: {
      ...eolEnum,
      description:
        "End-of-line character for SVG output. Inherited from dprint global newLineKind when unset.",
    },
    pretty: {
      type: "boolean",
      description: "Whether to pretty-print the SVG output.",
    },
    finalNewline: {
      type: "boolean",
      description: "Whether to add a final newline at the end of the output.",
    },
    useShortTags: {
      type: "boolean",
      description:
        "Whether to use short self-closing tags (e.g. `<path/>` instead of `<path></path>`).",
    },
    multipass: {
      type: "boolean",
      description:
        "Whether to enable multipass optimization. Multiple passes may produce a smaller output.",
    },
    plugins: {
      type: "array",
      description: "Array of SVGO plugin configurations.",
      items: pluginEntrySchema,
    },
    floatPrecision: {
      type: "integer",
      minimum: 0,
      maximum: 20,
      description: "Number of digits after the decimal point (0-20).",
    },
    datauri: {
      type: "string",
      enum: ["base64", "enc", "unenc"],
      description: "Type of Data URI encoding.",
    },
    js2svg: js2svgSchema,
    path: {
      type: "string",
      description: "Path to the SVG file (used by some SVGO plugins like prefixIds).",
    },
  },
  additionalProperties: true,
  _meta: {
    svgoVersion,
    generatedAt: new Date().toISOString(),
    presetDefault: [...defaultImportNames],
    pluginDescriptions,
  },
};

// ---------------------------------------------------------------------------
// 4. Output
// ---------------------------------------------------------------------------

const json = JSON.stringify(schema, null, 2) + "\n";
const outputPath = Deno.args[0];

if (outputPath) {
  await Deno.writeTextFile(outputPath, json);
  console.log(`\u2713 Wrote ${outputPath}`);
} else {
  console.log(json);
}

const defaultCount = allPlugins.filter((p) => defaultImportNames.has(p.name)).length;
console.error(
  `  ${presetNames.length} preset(s), ${allPlugins.length} plugins ` +
    `(${defaultCount} default, ${allPlugins.length - defaultCount} extra)`,
);
