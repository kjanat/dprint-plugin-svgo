const LATEST_URL = "https://plugins.dprint.dev/kjanat/svgo/latest.json";
const SCHEMA_URL = "https://plugins.dprint.dev/kjanat/svgo/latest/schema.json";

function highlight(json: string): string {
  return json.replace(
    /("(?:\\.|[^"\\])*")\s*:/g,
    '<span class="json-key">$1</span>:',
  ).replace(
    /:\s*("(?:\\.|[^"\\])*")/g,
    ': <span class="json-string">$1</span>',
  ).replace(
    /:\s*(-?\d+\.?\d*)/g,
    ': <span class="json-number">$1</span>',
  ).replace(
    /:\s*(true|false)/g,
    ': <span class="json-bool">$1</span>',
  ).replace(
    /:\s*(null)/g,
    ': <span class="json-null">$1</span>',
  ).replace(
    /([[\]{}])/g,
    '<span class="json-bracket">$1</span>',
  );
}

const meta = document.getElementById("meta");
const output = document.getElementById("json");

const [latest, schema] = await Promise.all([
  fetch(LATEST_URL).then((r) => r.json()),
  fetch(SCHEMA_URL).then((r) => r.json()),
]);

if (meta) {
  meta.textContent = `v${latest.version} \u00b7 checksum: ${latest.checksum.slice(0, 12)}\u2026`;
}

if (output) {
  output.innerHTML = highlight(JSON.stringify(schema, null, 2));
}
