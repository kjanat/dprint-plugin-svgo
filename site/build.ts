import { $, build, file } from "bun";
import { normalize } from "path";

const tag = process.env.TAG;
const schema = file(`${import.meta.dir}/schema.json`);

const outdir = normalize(`${import.meta.dir}/../dist`);

if (!(await schema.exists())) {
  if (tag) await $`gh release download ${tag} -p schema.json -O ${schema.name}`;
  else await $`gh release download -p schema.json -O ${schema.name}`;
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

const outfiles = result.outputs.map((o) => `    ${o.path} (${(o.size / 1024).toFixed(1)}KB)`);
console.log(`\nSite built:\t${outfiles.join("\n")}`);
