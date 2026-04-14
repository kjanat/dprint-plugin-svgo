import { parse as parseJsonc } from "jsr:@std/jsonc";
import { join } from "@std/path";
import { CargoToml, processPlugin } from "dprint/automation/mod.ts";
import { getChecksum } from "dprint/automation/hash.ts";
import $ from "dax";

export const GITHUB_OWNER = "kjanat";
export const PLUGIN_NAME = "dprint-plugin-svgo";
export const rootDirPath = $.path(import.meta.dirname!).parentOrThrow();
export const vendorSvgoDirPath = rootDirPath.join("vendor", "svgo");
export const releaseDirPath = rootDirPath.join("target", "release");
const denoConfigPath = rootDirPath.join("deno.jsonc");
const dprintConfigPath = rootDirPath.join(".dprint.jsonc");
const localTestSkippedDirNames = new Set([".git", "dist", "node_modules", "target", "vendor"]);

export async function buildJsBundle() {
  await $`deno bundle --frozen --format iife --platform browser --minify -o js/dist/svgo.js js/svgo.ts`
    .cwd(rootDirPath);
}

export async function cargoBuildRelease() {
  await $`cargo build --release`.cwd(rootDirPath);
}

export async function cargoTestAllFeatures() {
  await $`cargo test --all-features`.cwd(rootDirPath);
}

export async function refreshDenoLock() {
  await $`deno cache --frozen=false --lock=deno.lock js/svgo.ts scripts/create_plugin_file.ts scripts/generate_schema.ts scripts/local_test.ts scripts/output_svgo_version.ts scripts/update.ts .github/workflows/ci.generate.ts`
    .cwd(rootDirPath);
}

export async function syncSvgoDenoImports() {
  const configText = await Deno.readTextFile(denoConfigPath.toString());
  const config = parseJsonc(configText);
  if (!isRecord(config) || !isRecord(config.imports)) {
    throw new Error("Expected deno.jsonc to contain an imports object.");
  }

  const packageJson = await getSvgoPackageJson();
  const dependencies = packageJson.dependencies;
  if (!isRecord(dependencies)) {
    throw new Error("Expected vendor/svgo/package.json to contain a dependencies object.");
  }

  const imports = config.imports;
  const managedImportNames = new Set([...Object.keys(dependencies), "svgo/browser"]);

  // SVGO owns the npm import aliases in deno.jsonc, so drop any stale ones first.
  for (const [name, value] of Object.entries(imports)) {
    if (typeof value === "string" && value.startsWith("npm:") && !managedImportNames.has(name)) {
      delete imports[name];
    }
  }

  imports["svgo/browser"] = "./vendor/svgo/lib/svgo.js";

  for (const [name, version] of Object.entries(dependencies)) {
    if (typeof version !== "string") {
      throw new Error(`Expected dependency version for ${name} to be a string.`);
    }
    imports[name] = getSvgoDependencyImportSpecifier(name, version);
  }

  await Deno.writeTextFile(
    denoConfigPath.toString(),
    replaceObjectProperty(configText, "imports", imports),
  );
}

export async function getSvgoCompilerPaths() {
  const packageJson = await getSvgoPackageJson();
  const dependencies = packageJson.dependencies;
  if (!isRecord(dependencies)) {
    throw new Error("Expected vendor/svgo/package.json to contain a dependencies object.");
  }

  const entries = await Promise.all(
    Object.keys(dependencies).map(async (name) => {
      const { packageDir } = await getInstalledNpmPackageDir(name);
      const packageFilePath = join(packageDir, "package.json");
      const packageJson = JSON.parse(await Deno.readTextFile(packageFilePath)) as Record<
        string,
        unknown
      >;
      const targetPath = await resolvePackageTypesPath(packageDir, packageJson);
      return targetPath == null ? null : [name, [targetPath]] as const;
    }),
  );
  const filteredEntries = entries.filter(
    (entry): entry is readonly [string, readonly [string]] => entry != null,
  );

  return Object.fromEntries(filteredEntries);
}

export async function createPluginFile(
  options: { outputDir?: string; test?: boolean } = {},
) {
  const outputDir = options.outputDir ?? rootDirPath.toString();
  const isTest = options.test ?? false;
  const cargoFilePath = rootDirPath.join("Cargo.toml");
  const version = new CargoToml(cargoFilePath).version();
  const builder = new processPlugin.PluginFileBuilder({
    name: PLUGIN_NAME,
    version,
  });

  if (isTest) {
    const platform = processPlugin.getCurrentPlatform();
    const zipFileName = processPlugin.getStandardZipFileName(PLUGIN_NAME, platform);
    await builder.addPlatform({
      platform,
      zipFilePath: join(outputDir, zipFileName),
      zipUrl: zipFileName,
    });
  } else {
    const platforms: processPlugin.Platform[] = [
      "darwin-x86_64",
      "darwin-aarch64",
      "linux-x86_64",
      "linux-aarch64",
      "windows-x86_64",
    ];

    for (const platform of platforms) {
      const zipFileName = processPlugin.getStandardZipFileName(PLUGIN_NAME, platform);
      const zipUrl =
        `https://github.com/${GITHUB_OWNER}/${PLUGIN_NAME}/releases/download/${version}/${zipFileName}`;
      await builder.addPlatform({
        platform,
        zipFilePath: join(outputDir, zipFileName),
        zipUrl,
      });
    }
  }

  const pluginFilePath = join(outputDir, "plugin.json");
  await builder.writeToPath(pluginFilePath);
  return pluginFilePath;
}

export async function prepareLocalTestArtifacts() {
  await buildJsBundle();
  await cargoBuildRelease();
  await zipCurrentPlatformReleaseBinary();
  const pluginFilePath = await createPluginFile({
    outputDir: releaseDirPath.toString(),
    test: true,
  });
  const checksum = await getFileChecksum(pluginFilePath);
  return {
    checksum,
    pluginFilePath,
  };
}

export async function createLocalTestWorkspace(pluginReference: string) {
  const tempDirPath = await Deno.makeTempDir({
    dir: rootDirPath.join("target").toString(),
    prefix: "local-test-",
  });
  const workspaceDirPath = join(tempDirPath, "workspace");
  await copyDirectory(rootDirPath.toString(), workspaceDirPath);
  const configPath = await writeLocalTestConfig(pluginReference, workspaceDirPath);
  return {
    configPath,
    tempDirPath,
    workspaceDirPath,
  };
}

export async function runDprintFormat(configPath: string) {
  await $`dprint fmt --config ${configPath}`.cwd($.path(configPath).parentOrThrow());
}

export async function getSvgoVersion() {
  const packageJson = await getSvgoPackageJson();
  return packageJson.version as string;
}

async function zipCurrentPlatformReleaseBinary() {
  const platform = processPlugin.getCurrentPlatform();
  const zipFileName = processPlugin.getStandardZipFileName(PLUGIN_NAME, platform);
  const binaryName = Deno.build.os === "windows" ? `${PLUGIN_NAME}.exe` : PLUGIN_NAME;
  const zipFilePath = join(releaseDirPath.toString(), zipFileName);

  await Deno.remove(zipFilePath).catch((error) => {
    if (!(error instanceof Deno.errors.NotFound)) throw error;
  });

  if (Deno.build.os === "windows") {
    await $`powershell -Command ${`Compress-Archive -Force -Path target/release/${binaryName} -DestinationPath target/release/${zipFileName}`}`
      .cwd(rootDirPath);
  } else {
    await $`zip -j ${zipFileName} ${binaryName}`.cwd(releaseDirPath);
  }

  return zipFilePath;
}

async function getFileChecksum(filePath: string) {
  const bytes = await Deno.readFile(filePath);
  return await getChecksum(bytes);
}

async function writeLocalTestConfig(pluginReference: string, workspaceDirPath: string) {
  const configText = await Deno.readTextFile(dprintConfigPath.toString());
  const config = parseJsonc(configText);
  if (!isRecord(config)) {
    throw new Error("Expected .dprint.jsonc to contain an object.");
  }

  const exec = config.exec;
  if (!isRecord(exec)) {
    throw new Error("Expected .dprint.jsonc exec config to be an object.");
  }
  exec.cwd = workspaceDirPath;

  if (!Array.isArray(config.plugins)) {
    throw new Error("Expected .dprint.jsonc plugins to be an array.");
  }

  const pluginIndex = config.plugins.findIndex((plugin) =>
    typeof plugin === "string" && plugin.includes("plugins.dprint.dev/kjanat/svg-v")
  );
  if (pluginIndex === -1) {
    throw new Error("Could not find the published svgo plugin entry in .dprint.jsonc.");
  }
  config.plugins[pluginIndex] = pluginReference;

  const configPath = join(workspaceDirPath, ".dprint.jsonc");
  await Deno.writeTextFile(configPath, `${JSON.stringify(config, null, 2)}\n`);
  return configPath;
}

async function copyDirectory(sourceDirPath: string, destinationDirPath: string) {
  await Deno.mkdir(destinationDirPath, { recursive: true });

  for await (const entry of Deno.readDir(sourceDirPath)) {
    if (entry.isDirectory && localTestSkippedDirNames.has(entry.name)) {
      continue;
    }

    const sourcePath = join(sourceDirPath, entry.name);
    const destinationPath = join(destinationDirPath, entry.name);

    if (entry.isDirectory) {
      await copyDirectory(sourcePath, destinationPath);
      continue;
    }

    if (entry.isFile) {
      await Deno.copyFile(sourcePath, destinationPath);
      continue;
    }

    if (entry.isSymlink) {
      const targetPath = await Deno.readLink(sourcePath);
      await Deno.symlink(targetPath, destinationPath);
    }
  }
}

async function getSvgoPackageJson() {
  return JSON.parse(await Deno.readTextFile(vendorSvgoDirPath.join("package.json").toString()));
}

async function getInstalledNpmPackageDir(packageName: string) {
  const infoText = await $`deno info --json ${packageName}`.cwd(rootDirPath).text();
  const info = JSON.parse(infoText) as {
    modules?: Array<{ kind?: string; npmPackage?: string }>;
    npmPackages?: Record<string, { registryUrl?: string }>;
  };
  const npmPackage = info.modules?.find((module) =>
    module.kind === "npm" && typeof module.npmPackage === "string"
  )?.npmPackage;
  if (npmPackage == null) {
    throw new Error(`Could not resolve installed npm package for ${packageName}.`);
  }

  const versionSeparatorIndex = npmPackage.lastIndexOf("@");
  if (versionSeparatorIndex <= 0) {
    throw new Error(`Unexpected npm package identifier: ${npmPackage}`);
  }

  const resolvedName = npmPackage.slice(0, versionSeparatorIndex);
  const resolvedVersion = npmPackage.slice(versionSeparatorIndex + 1);
  const registryUrl = info.npmPackages?.[npmPackage]?.registryUrl;
  const registryHost = getRegistryHost(registryUrl);
  const denoDirPath = Deno.env.get("DENO_DIR") ?? getDefaultDenoCacheDir();
  return {
    packageDir: join(
      denoDirPath,
      "npm",
      registryHost,
      ...resolvedName.split("/"),
      resolvedVersion,
    ),
  };
}

async function resolvePackageTypesPath(packageDir: string, packageJson: Record<string, unknown>) {
  const candidatePaths = [
    resolvePackageTypesEntry(packageJson),
    "index.d.ts",
  ].flatMap((candidate) => candidate == null ? [] : [join(packageDir, candidate)]);

  for (const candidatePath of candidatePaths) {
    if (await pathExists(candidatePath)) {
      return candidatePath;
    }
  }
}

function resolvePackageTypesEntry(packageJson: Record<string, unknown>) {
  const types = packageJson.types;
  if (typeof types === "string") {
    return types;
  }

  const typings = packageJson.typings;
  if (typeof typings === "string") {
    return typings;
  }

  const exports = packageJson.exports;
  if (!isRecord(exports)) {
    return undefined;
  }

  const mainExport = exports["."];
  if (!isRecord(mainExport)) {
    return undefined;
  }

  const exportTypes = mainExport.types;
  return typeof exportTypes === "string" ? exportTypes : undefined;
}

function getDefaultDenoCacheDir() {
  switch (Deno.build.os) {
    case "darwin":
      return join(Deno.env.get("HOME") ?? "", "Library", "Caches", "deno");
    case "windows":
      return join(Deno.env.get("LOCALAPPDATA") ?? "", "deno");
    default:
      return join(Deno.env.get("HOME") ?? "", ".cache", "deno");
  }
}

function getRegistryHost(registryUrl?: string) {
  if (registryUrl == null) {
    return "registry.npmjs.org";
  }

  const url = new URL(registryUrl);
  return url.host;
}

function getSvgoDependencyImportSpecifier(name: string, version: string) {
  switch (name) {
    case "css-tree":
      return `npm:${name}@${version}/dist/csstree.esm`;
    case "csso":
      return `npm:${name}@${version}/dist/csso.esm`;
    default:
      return `npm:${name}@${version}`;
  }
}

async function pathExists(filePath: string) {
  try {
    await Deno.stat(filePath);
    return true;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      return false;
    }
    throw error;
  }
}

function replaceObjectProperty(
  sourceText: string,
  propertyName: string,
  propertyValue: Record<string, unknown>,
) {
  const propertyToken = `"${propertyName}"`;
  const keyStart = sourceText.indexOf(propertyToken);
  if (keyStart === -1) {
    throw new Error(`Could not find ${propertyToken} in deno.jsonc.`);
  }

  const lineStart = sourceText.lastIndexOf("\n", keyStart) + 1;
  const indent = sourceText.slice(lineStart, keyStart);
  const colonIndex = sourceText.indexOf(":", keyStart + propertyToken.length);
  if (colonIndex === -1) {
    throw new Error(`Could not find ':' for ${propertyToken} in deno.jsonc.`);
  }

  const objectStart = sourceText.indexOf("{", colonIndex + 1);
  if (objectStart === -1) {
    throw new Error(`Could not find object start for ${propertyToken} in deno.jsonc.`);
  }

  const objectEnd = findMatchingBrace(sourceText, objectStart);
  const serializedObject = JSON.stringify(propertyValue, null, 2)
    .split("\n")
    .map((line, index) => index === 0 ? line : `${indent}${line}`)
    .join("\n");
  return `${sourceText.slice(0, keyStart)}${propertyToken}: ${serializedObject}${
    sourceText.slice(objectEnd + 1)
  }`;
}

function findMatchingBrace(sourceText: string, openBraceIndex: number) {
  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let i = openBraceIndex; i < sourceText.length; i++) {
    const char = sourceText[i];
    if (escaped) {
      escaped = false;
      continue;
    }

    if (char === "\\") {
      escaped = true;
      continue;
    }

    if (char === '"') {
      inString = !inString;
      continue;
    }

    if (inString) {
      continue;
    }

    if (char === "{") depth++;
    if (char === "}") depth--;
    if (depth === 0) return i;
  }

  throw new Error("Could not find matching closing brace in deno.jsonc.");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
