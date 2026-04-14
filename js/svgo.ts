import { optimize } from "svgo/browser";
import type { Config } from "../vendor/svgo/lib/types.ts";

declare global {
  var dprint: {
    getExtensions: typeof getExtensions;
    formatText: typeof formatText;
  };
}

globalThis.dprint = {
  getExtensions,
  formatText,
};

function getExtensions() {
  return ["svg"];
}

interface FormatTextOptions {
  filePath: string;
  fileText: string;
  config: Config;
}

function formatText(
  { filePath, fileText, config }: FormatTextOptions,
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
    const fileName = filePath.split(/[/\\]/).pop() || filePath;
    const errorMessage = error instanceof Error ? error.message : "Unknown error";
    console.error(`SVGO error for ${fileName}: ${errorMessage}`);
    return undefined;
  }
}
