import { parse as parseJsonc } from "jsr:@std/jsonc";
import { dirname, join } from "@std/path";
import { CargoToml, processPlugin } from "dprint/automation/mod.ts";
import { getChecksum } from "dprint/automation/hash.ts";
import $ from "dax";

export const GITHUB_OWNER = "kjanat";
export const PLUGIN_NAME = "dprint-plugin-svgo";
export const rootDirPath = $.path(import.meta.dirname!).parentOrThrow();
export const releaseDirPath = rootDirPath.join("target", "release");
const dprintConfigPath = rootDirPath.join(".dprint.jsonc");
const localTestSkippedDirNames = new Set([".git", "dist", "node_modules", "target"]);

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
  await $`dprint fmt --config ${configPath}`.cwd(dirname(configPath));
}

export async function getSvgoDir() {
  const svgoBrowserUrl = import.meta.resolve("svgo/browser");
  return new URL("..", svgoBrowserUrl).pathname;
}

export async function getSvgoVersion() {
  const svgoDir = await getSvgoDir();
  const packageJson = JSON.parse(await Deno.readTextFile(join(svgoDir, "package.json")));
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
