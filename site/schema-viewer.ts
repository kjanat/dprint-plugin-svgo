import schema from "./schema.json" with { type: "json" };

interface SchemaWithId {
  $id?: string;
  _meta?: {
    pluginVersion?: string;
  };
}

function esc(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function highlight(json: string): string {
  return esc(json)
    .replace(
      /("(?:\\.|[^"\\])*")\s*:/g,
      '<span class="json-key">$1</span>:',
    )
    .replace(
      /:\s*("(?:\\.|[^"\\])*")/g,
      ': <span class="json-string">$1</span>',
    )
    .replace(
      /:\s*(-?\d+\.?\d*)/g,
      ': <span class="json-number">$1</span>',
    )
    .replace(
      /:\s*(true|false)/g,
      ': <span class="json-bool">$1</span>',
    )
    .replace(
      /:\s*(null)/g,
      ': <span class="json-null">$1</span>',
    )
    .replace(
      /([[\]{}])/g,
      '<span class="json-bracket">$1</span>',
    );
}

const schemaWithId = schema as SchemaWithId;
const version = schemaWithId._meta?.pluginVersion ??
  schemaWithId.$id?.match(/\/(\d+\.\d+\.\d+)\//)?.[1];
const meta = document.getElementById("meta");
const output = document.getElementById("json");

if (meta) {
  meta.textContent = version ? `v${version}` : "";
}

if (output) {
  output.innerHTML = highlight(JSON.stringify(schema, null, 2));
}
