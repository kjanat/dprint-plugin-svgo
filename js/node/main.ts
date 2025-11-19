import { type Config, optimize } from "svgo/browser";

(globalThis as any).dprint = {
  getExtensions,
  formatText,
};

async function getExtensions() {
  // SVGO only supports SVG files
  return ["svg"];
}

interface FormatTextOptions {
  filePath: string;
  fileText: string;
  config: Config;
  pluginsConfig: PluginsConfig;
}

interface PluginsConfig {
  // SVGO-specific plugin configuration can be added here
}

async function formatText(
  { filePath, fileText, config, pluginsConfig }: FormatTextOptions,
) {
  try {
    const result = optimize(fileText, {
      path: filePath,
      ...config,
    });

    const formattedText = result.data;
    if (formattedText === fileText) {
      return undefined;
    } else {
      return formattedText;
    }
  } catch (error) {
    // If SVGO fails to optimize (e.g., invalid SVG), return undefined to keep original
    // Sanitize error message to avoid leaking internal paths
    const fileName = filePath.split(/[/\\]/).pop() || filePath;
    const errorMessage = error instanceof Error
      ? error.message
      : "Unknown error";
    console.error(`SVGO error for ${fileName}: ${errorMessage}`);
    return undefined;
  }
}
