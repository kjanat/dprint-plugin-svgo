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

interface TypeProperty {
  name: string;
  value: string;
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

function extractTypeBody(source: string, typeName: string): string {
  const typeStart = source.indexOf(`export type ${typeName} =`);
  if (typeStart === -1) {
    throw new Error(`Could not find type ${typeName} in SVGO lib/types.ts`);
  }

  const bodyStart = source.indexOf("{", typeStart);
  if (bodyStart === -1) {
    throw new Error(`Could not find body for type ${typeName} in SVGO lib/types.ts`);
  }

  let depth = 0;
  for (let i = bodyStart; i < source.length; i++) {
    const char = source[i];
    if (char === "{") depth++;
    if (char === "}") depth--;
    if (depth === 0) return source.slice(bodyStart + 1, i);
  }

  throw new Error(`Unterminated body for type ${typeName} in SVGO lib/types.ts`);
}

function stripComments(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

function parseTypeProperties(typeBody: string): TypeProperty[] {
  const sanitized = stripComments(typeBody);
  const properties: TypeProperty[] = [];
  let depth = 0;
  let segmentStart = 0;

  for (let i = 0; i < sanitized.length; i++) {
    const char = sanitized[i];
    if (char === "{") depth++;
    if (char === "}") depth--;
    if (char !== ";" || depth !== 0) continue;

    const segment = sanitized.slice(segmentStart, i).trim();
    segmentStart = i + 1;
    if (!segment) continue;

    const match = segment.match(/^(?:'([^']+)'|([A-Za-z0-9_-]+))\??:\s*([\s\S]+)$/);
    if (match) {
      properties.push({
        name: match[1] ?? match[2],
        value: match[3].trim(),
      });
    }
  }

  return properties;
}

function classifyPluginParams(properties: TypeProperty[]): Map<string, "null" | "object"> {
  return new Map(
    properties.map((property) => [
      property.name,
      property.value === "null" ? "null" : "object",
    ]),
  );
}

function assertSameNames(actual: Iterable<string>, expected: Iterable<string>, label: string) {
  const actualSet = new Set(actual);
  const expectedSet = new Set(expected);
  const missing = [...expectedSet].filter((name) => !actualSet.has(name));
  const extra = [...actualSet].filter((name) => !expectedSet.has(name));

  if (missing.length === 0 && extra.length === 0) return;

  throw new Error(
    `${label} mismatch. Missing: ${missing.join(", ") || "none"}. ` +
      `Extra: ${extra.join(", ") || "none"}.`,
  );
}

function createObjectParamsSchema(description: string) {
  return {
    type: "object" as const,
    additionalProperties: true,
    description,
  };
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

const typesSource = await Deno.readTextFile(join(SVGO_DIR, "lib/types.ts"));
const defaultPluginProperties = parseTypeProperties(extractTypeBody(typesSource, "DefaultPlugins"));
const optionalExtraProperties = parseTypeProperties(
  extractTypeBody(typesSource, "BuiltinsWithOptionalParams"),
);
const requiredPluginProperties = parseTypeProperties(
  extractTypeBody(typesSource, "BuiltinsWithRequiredParams"),
);

const defaultPluginParamKinds = classifyPluginParams(defaultPluginProperties);
const optionalPluginParamKinds = new Map<string, "null" | "object">([
  ...classifyPluginParams(defaultPluginProperties),
  ...classifyPluginParams(optionalExtraProperties),
]);
const requiredPluginNameSet = new Set(requiredPluginProperties.map((property) => property.name));

const builtinPluginNames = allPlugins.map((plugin) => plugin.name);
const optionalPluginNames = [
  ...presetNames.filter((name) => optionalPluginParamKinds.has(name)),
  ...builtinPluginNames.filter((name) => optionalPluginParamKinds.has(name)),
];
const requiredPluginNames = builtinPluginNames.filter((name) => requiredPluginNameSet.has(name));
const nullParamPluginNames = builtinPluginNames.filter((name) =>
  optionalPluginParamKinds.get(name) === "null"
);
const optionalObjectParamPluginNames = builtinPluginNames.filter(
  (name) => optionalPluginParamKinds.get(name) === "object",
);

assertSameNames(
  [...optionalPluginNames, ...requiredPluginNames],
  [...presetNames, ...builtinPluginNames],
  "SVGO plugin classification",
);
assertSameNames(
  defaultPluginProperties.map((property) => property.name),
  defaultImportNames,
  "SVGO preset-default plugins",
);

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

const pluginDescriptions = Object.fromEntries(
  allPlugins.map((p) => [p.name, p.description]),
);

const genericPluginParamsSchema = createObjectParamsSchema(
  "Plugin parameters. Exact keys depend on the plugin.",
);

const presetDefaultOverrideSchemas = Object.fromEntries(
  defaultPluginProperties.map((property) => {
    const schema = defaultPluginParamKinds.get(property.name) === "null"
      ? {
        oneOf: [
          { type: "null" as const },
          { const: false },
        ],
      }
      : {
        oneOf: [
          createObjectParamsSchema(`Override parameters for ${property.name}.`),
          { const: false },
        ],
      };
    return [property.name, schema];
  }),
);

const presetDefaultParamsSchema = {
  type: "object" as const,
  properties: {
    floatPrecision: {
      type: "integer" as const,
      minimum: 0,
      maximum: 20,
      description: "Number of digits after the decimal point (0-20).",
    },
    overrides: {
      type: "object" as const,
      description: "Override or disable plugins included by preset-default.",
      properties: presetDefaultOverrideSchemas,
      additionalProperties: false,
    },
  },
  additionalProperties: false,
  description:
    "Preset parameters. Use `overrides` to disable or configure plugins included in preset-default.",
};

const pluginEntrySchema = {
  oneOf: [
    {
      type: "string" as const,
      enum: optionalPluginNames,
      description:
        "Built-in plugin or preset referenced by name. Only valid when params are optional.",
    },
    {
      type: "object" as const,
      required: ["name"],
      properties: {
        name: {
          type: "string" as const,
          enum: ["preset-default"],
          description: "Preset name.",
        },
        params: {
          ...presetDefaultParamsSchema,
        },
      },
      additionalProperties: false,
    },
    {
      type: "object" as const,
      required: ["name"],
      properties: {
        name: {
          type: "string" as const,
          enum: nullParamPluginNames,
          description: "Plugin name.",
        },
        params: {
          type: "null" as const,
          description: "Param-less plugin. Omit `params` or set it to `null`.",
        },
      },
      additionalProperties: false,
    },
    {
      type: "object" as const,
      required: ["name"],
      properties: {
        name: {
          type: "string" as const,
          enum: optionalObjectParamPluginNames,
          description: "Plugin name.",
        },
        params: genericPluginParamsSchema,
      },
      additionalProperties: false,
    },
    {
      type: "object" as const,
      required: ["name", "params"],
      properties: {
        name: {
          type: "string" as const,
          enum: requiredPluginNames,
          description: "Plugin name.",
        },
        params: {
          ...genericPluginParamsSchema,
          description: "Plugin parameters. This plugin requires a params object.",
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
      description:
        "Array of SVGO built-in plugin configurations. Custom JavaScript plugins are not supported in dprint config.",
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
