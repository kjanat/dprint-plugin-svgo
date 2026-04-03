import schema from "./schema.json" with { type: "json" };

interface SchemaProperty {
  type?: string;
  enum?: string[];
  description?: string;
  minimum?: number;
  maximum?: number;
  properties?: Record<string, SchemaProperty>;
  items?: { oneOf?: Array<{ enum?: string[] }> };
}

interface Schema {
  $id?: string;
  properties?: Record<string, SchemaProperty>;
  _meta?: {
    presetDefault?: string[];
    pluginDescriptions?: Record<string, string>;
  };
}

const s = schema as Schema;

// --- DOM helpers ---

const el = (
  tag: string,
  attrs: Record<string, string> | null = null,
  ...children: (string | Node)[]
): HTMLElement => {
  const node = document.createElement(tag);
  if (attrs) { for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v); }
  for (const c of children) typeof c === "string" ? node.append(c) : node.appendChild(c);
  return node;
};

const code = (t: string) => el("code", null, t);

function heading(text: string, id: string): HTMLHeadingElement {
  const h = el("h2", { id }) as HTMLHeadingElement;
  h.append(text);
  h.appendChild(el("a", { class: "anchor", href: `#${id}` }, "#"));
  return h;
}

// --- JSON syntax highlighting ---

function highlightJson(json: string): string {
  return json.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/"([^"]+)"(?=\s*:)/g, '<span class="json-key">"$1"</span>')
    .replace(/:\s*"([^"]*)"/g, ': <span class="json-string">"$1"</span>')
    .replace(/:\s*(\d+)/g, ': <span class="json-number">$1</span>')
    .replace(/:\s*(true|false)/g, ': <span class="json-bool">$1</span>');
}

// --- Components ---

function propRow(name: string, prop: SchemaProperty): HTMLTableRowElement {
  const tr = el("tr") as HTMLTableRowElement;
  tr.appendChild(el("td", null, code(name)));

  const type = prop.type || (prop.enum ? "enum" : "object");
  const range = [
    prop.minimum !== undefined ? `min: ${prop.minimum}` : "",
    prop.maximum !== undefined ? `max: ${prop.maximum}` : "",
  ].filter(Boolean).join(", ");
  tr.appendChild(el("td", null, type + (range ? ` (${range})` : "")));

  const desc = el("td", null, prop.description || "");
  if (prop.enum) {
    desc.appendChild(document.createElement("br"));
    for (const v of prop.enum) {
      desc.appendChild(el("span", { class: "chip" }, v));
    }
  }
  tr.appendChild(desc);
  return tr;
}

function propsTable(
  properties: Record<string, SchemaProperty>,
  exclude: string[],
): HTMLTableElement {
  const table = el("table") as HTMLTableElement;
  const thead = el("thead");
  const hr = el("tr");
  for (const h of ["Option", "Type", "Description"]) hr.appendChild(el("th", null, h));
  thead.appendChild(hr);
  table.appendChild(thead);
  const tbody = el("tbody");
  for (const [k, v] of Object.entries(properties)) {
    if (!exclude.includes(k)) tbody.appendChild(propRow(k, v));
  }
  table.appendChild(tbody);
  return table;
}

function pluginList(
  items: string[],
  descriptions: Record<string, string>,
  title: string,
): DocumentFragment {
  const frag = document.createDocumentFragment();
  if (!items.length) return frag;
  const h = el("h3");
  h.innerHTML = `${title} <span class="count">(${items.length})</span>`;
  frag.appendChild(h);
  const ul = el("ul", { class: "plugins" });
  for (const p of items) {
    const li = el("li");
    li.setAttribute("data-plugin", p.toLowerCase());
    li.appendChild(code(p));
    if (descriptions[p]) li.append(` \u2014 ${descriptions[p]}`);
    ul.appendChild(li);
  }
  frag.appendChild(ul);
  return frag;
}

// --- Copy button ---

const copyBtn = document.getElementById("copy-btn");
copyBtn?.addEventListener("click", () => {
  navigator.clipboard.writeText("dprint add kjanat/svgo").then(() => {
    if (copyBtn) copyBtn.textContent = "Copied!";
    setTimeout(() => {
      if (copyBtn) copyBtn.textContent = "Copy";
    }, 1500);
  });
});

// --- Render ---

const version = s.$id?.match(/\/(\d+\.\d+\.\d+)\//)?.[1];
if (version) {
  const vEl = document.getElementById("version");
  vEl?.insertBefore(document.createTextNode(`v${version} \u00b7 `), vEl.firstChild);
}

const content = document.getElementById("content");
if (!content) throw new Error("missing #content");
content.innerHTML = "";
const props = s.properties || {};

// Configuration table
content.appendChild(heading("Configuration", "configuration"));
content.appendChild(propsTable(props, ["plugins", "js2svg", "path"]));

// js2svg as collapsible
if (props.js2svg?.properties) {
  const details = el("details");
  const summary = el(
    "summary",
    null,
    "js2svg options (overrides top-level indent, eol, pretty, finalNewline, useShortTags)",
  );
  details.appendChild(summary);
  details.appendChild(propsTable(props.js2svg.properties, []));
  content.appendChild(details);
}

// Plugins with search
const plugins = props.plugins?.items?.oneOf?.[0]?.enum || [];
const defaults = new Set(s._meta?.presetDefault || []);
const descriptions = s._meta?.pluginDescriptions || {};
const defaultPlugins = plugins.filter((p) => defaults.has(p) || p === "preset-default");
const extraPlugins = plugins.filter((p) => !defaults.has(p) && p !== "preset-default");

content.appendChild(heading("Plugins", "plugins"));
const search = el("input", {
  class: "search-box",
  type: "text",
  placeholder: `Search ${plugins.length} plugins\u2026`,
}) as HTMLInputElement;
content.appendChild(search);

const pluginsContainer = el("div", { id: "plugins-list" });
pluginsContainer.appendChild(pluginList(defaultPlugins, descriptions, "Default (preset-default)"));
pluginsContainer.appendChild(pluginList(extraPlugins, descriptions, "Additional"));
content.appendChild(pluginsContainer);

search.addEventListener("input", () => {
  const q = search.value.toLowerCase();
  for (const li of pluginsContainer.querySelectorAll("li[data-plugin]")) {
    (li as HTMLElement).style.display = (li as HTMLElement).dataset.plugin?.includes(q)
      ? ""
      : "none";
  }
});

// Example with syntax highlighting
content.appendChild(heading("Example", "example"));
const pre = el("pre");
const codeEl = el("code");
codeEl.innerHTML = highlightJson(JSON.stringify(
  {
    svgo: {
      multipass: true,
      pretty: true,
      indent: 2,
      plugins: [
        "preset-default",
        { name: "removeViewBox", params: {} },
        { name: "prefixIds", params: { prefix: "icon" } },
      ],
    },
  },
  null,
  2,
));
pre.appendChild(codeEl);
content.appendChild(pre);

// Footer
const footer = document.getElementById("footer");
if (footer) {
  footer.innerHTML = [
    version ? `v${version}` : "",
    'Powered by <a href="https://svgo.dev">SVGO</a>',
    '<a href="schema-viewer.html">schema.json</a>',
  ].filter(Boolean).join(" \u00b7 ");
}
