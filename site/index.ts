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

function propRow(name: string, prop: SchemaProperty): HTMLTableRowElement {
  const tr = el("tr") as HTMLTableRowElement;
  tr.appendChild(el("td", null, code(name)));
  const type = prop.type || (prop.enum ? "enum" : "object");
  const range = [
    prop.minimum !== undefined ? `min: ${prop.minimum}` : "",
    prop.maximum !== undefined ? `max: ${prop.maximum}` : "",
  ]
    .filter(Boolean)
    .join(", ");
  tr.appendChild(el("td", null, type + (range ? ` (${range})` : "")));
  const desc = el("td", null, prop.description || "");
  if (prop.enum) {
    desc.appendChild(document.createElement("br"));
    desc.append(
      prop.enum
        .map((v) => code(v))
        .reduce((a, b) => {
          a.append(" | ");
          a.appendChild(b);
          return a;
        }, el("span")),
    );
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
): DocumentFragment | HTMLSpanElement {
  if (!items.length) return el("span");
  const frag = document.createDocumentFragment();
  const h = el("h3");
  h.innerHTML = `${title} <span class="count">(${items.length})</span>`;
  frag.appendChild(h);
  const ul = el("ul", { class: "plugins" });
  for (const p of items) {
    const li = el("li");
    li.appendChild(code(p));
    if (descriptions[p]) li.append(` \u2014 ${descriptions[p]}`);
    ul.appendChild(li);
  }
  frag.appendChild(ul);
  return frag;
}

const version = (schema as Schema).$id?.match(/\/(\d+\.\d+\.\d+)\//)?.[1];
if (version) {
  const vEl = document.getElementById("version");
  vEl?.insertBefore(document.createTextNode(`v${version} \u00b7 `), vEl.firstChild);
}

const content = document.getElementById("content")!;
content.innerHTML = "";

const props = (schema as Schema).properties || {};

content.appendChild(el("h2", null, "Configuration"));
content.appendChild(propsTable(props, ["plugins", "js2svg", "path"]));

if (props.js2svg?.properties) {
  content.appendChild(el("h2", null, "js2svg options"));
  content.appendChild(propsTable(props.js2svg.properties, []));
}

const plugins = props.plugins?.items?.oneOf?.[0]?.enum || [];
const defaults = new Set((schema as Schema)._meta?.presetDefault || []);
const descriptions = (schema as Schema)._meta?.pluginDescriptions || {};

const defaultPlugins = plugins.filter((p) => defaults.has(p) || p === "preset-default");
const extraPlugins = plugins.filter((p) => !defaults.has(p) && p !== "preset-default");

content.appendChild(el("h2", null, "Plugins"));
content.appendChild(pluginList(defaultPlugins, descriptions, "Default (preset-default)"));
content.appendChild(pluginList(extraPlugins, descriptions, "Additional"));

content.appendChild(el("h2", null, "Example"));
const pre = el("pre");
pre.appendChild(
  el(
    "code",
    null,
    JSON.stringify(
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
    ),
  ),
);
content.appendChild(pre);
