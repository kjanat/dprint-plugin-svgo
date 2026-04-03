import schema from "./schema.json" with { type: "json" };

interface SchemaWithId {
  $id?: string;
}

function highlight(json: string): string {
  return json
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

const version = (schema as SchemaWithId).$id?.match(/\/(\d+\.\d+\.\d+)\//)?.[1];
const meta = document.getElementById("meta");
const output = document.getElementById("json");

if (meta) {
  meta.textContent = version ? `v${version}` : "";
}

if (output) {
  output.innerHTML = highlight(JSON.stringify(schema, null, 2));
}
