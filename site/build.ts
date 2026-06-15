import { build } from "bun";
import { access, copyFile, mkdir } from "node:fs/promises";
import { join, normalize } from "node:path";

const rootDir = normalize(`${import.meta.dir}/..`);
const schemaPath = join(import.meta.dir, "schema.json");
const outdir = join(rootDir, "dist");

try {
  await access(schemaPath);
} catch (error) {
  const suffix = error instanceof Error ? ` (${error.message})` : "";
  console.error(`schema.json not found at ${schemaPath}${suffix}`);
  console.error("Run `just site-schema` first.");
  process.exit(1);
}

const result = await build({
  entrypoints: [`${import.meta.dir}/index.html`, `${import.meta.dir}/schema-viewer.html`],
  outdir,
  minify: true,
  compile: true,
  target: "browser",
  publicPath: process.env.CI === "1" ? "https://dprint-svgo.kjanat.com/" : "/",
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
});

if (!result.success) {
  console.error("Build failed:");
  for (const log of result.logs) console.error(log);
  process.exit(1);
}

await mkdir(outdir, { recursive: true });
const outputSchemaPath = join(outdir, "schema.json");
await copyFile(schemaPath, outputSchemaPath);

const outfiles = [
  ...result.outputs.map((o) => `    ${o.path} (${(o.size / 1024).toFixed(1)}KB)`),
  `    ${outputSchemaPath}`,
];
console.log(`\nSite built:\t${outfiles.join("\n")}`);
