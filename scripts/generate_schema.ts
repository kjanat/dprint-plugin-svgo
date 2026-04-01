#!/usr/bin/env -S deno run -A
import $ from "dax";

const rootDir = $.path(import.meta.dirname!).parentOrThrow();
const cargoToml = await Deno.readTextFile(
  rootDir.join("plugin/Cargo.toml").toString(),
);
const versionMatch = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
if (!versionMatch) {
  throw new Error("Could not find version in plugin/Cargo.toml");
}
const version = versionMatch[1];

const schema = {
  $schema: "http://json-schema.org/draft-07/schema#",
  $id:
    `https://plugins.dprint.dev/kjanat/dprint-plugin-svgo/${version}/schema.json`,
  title: "dprint-plugin-svgo configuration",
  description:
    "Schema for dprint-plugin-svgo configuration. All fields are optional.",
  type: "object",
  properties: {
    indent: {
      description:
        "Number of spaces for indentation in SVG output. Inherited from dprint global indentWidth when unset.",
      type: "integer",
      default: 2,
      minimum: 0,
    },
    eol: {
      description:
        "End-of-line character for SVG output. Inherited from dprint global newLineKind when unset.",
      type: "string",
      oneOf: [
        { const: "lf", description: "Unix-style line feed." },
        {
          const: "crlf",
          description: "Windows-style carriage return + line feed.",
        },
      ],
    },
    pretty: {
      description: "Whether to pretty-print the SVG output.",
      type: "boolean",
      default: true,
    },
    multipass: {
      description:
        "Whether to enable multipass optimization. Multiple passes may produce a smaller output.",
      type: "boolean",
      default: false,
    },
    plugins: {
      description: "Array of SVGO plugin configurations.",
      type: "array",
      items: {
        oneOf: [
          { type: "string", description: "Plugin name as a string." },
          {
            type: "object",
            properties: {
              name: { type: "string", description: "Plugin name." },
              params: { type: "object", description: "Plugin parameters." },
            },
            required: ["name"],
          },
        ],
      },
    },
    floatPrecision: {
      description: "Number of digits after the decimal point (0\u201320).",
      type: "integer",
      minimum: 0,
      maximum: 20,
    },
    datauri: {
      description: "Type of Data URI encoding.",
      type: "string",
      oneOf: [
        { const: "base64", description: "Base64 encoding." },
        { const: "enc", description: "URL-safe encoding." },
        { const: "unenc", description: "Unencoded." },
      ],
    },
    js2svg: {
      description:
        "Direct js2svg configuration object passed to SVGO. Overrides indent, eol, and pretty when set.",
      type: "object",
    },
    path: {
      description: "Path to the SVG file (used by some SVGO plugins).",
      type: "string",
    },
  },
  additionalProperties: {
    description:
      "Extension-specific overrides use dotted keys (e.g. 'svg.multipass': false).",
  },
};

await Deno.writeTextFile("schema.json", JSON.stringify(schema, null, 2) + "\n");
console.log(`Generated schema.json for version ${version}`);
