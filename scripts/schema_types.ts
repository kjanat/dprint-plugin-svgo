import type {
  BuiltinsWithOptionalParams as SvgoBuiltinsWithOptionalParams,
  BuiltinsWithRequiredParams as SvgoBuiltinsWithRequiredParams,
  Config as SvgoConfig,
  DataUri,
  StringifyOptions as SvgoStringifyOptions,
} from "../vendor/svgo/lib/types.ts";

/** A selector rule for the removeAttributesBySelector plugin. */
interface RemoveAttributesBySelectorRule {
  /** CSS selector used to find matching SVG elements. */
  selector: string;
  /** One or more attribute names to remove from each matched element. */
  attributes: string | string[];
}

/** JSON-safe params for removeAttributesBySelector. */
type RemoveAttributesBySelectorParams =
  | RemoveAttributesBySelectorRule
  | {
    /** Multiple selector rules to apply. */
    selectors: RemoveAttributesBySelectorRule[];
  };

/** JSON-safe js2svg options supported by dprint-plugin-svgo. */
type DprintStringifyOptions = Omit<
  SvgoStringifyOptions,
  "regEntities" | "regValEntities" | "encodeEntity"
>;

/** Built-in plugins whose params remain optional in dprint config. */
type DprintBuiltinsWithOptionalParams =
  & Omit<
    SvgoBuiltinsWithOptionalParams,
    "prefixIds" | "convertColors" | "removeComments"
  >
  & {
    prefixIds: Omit<SvgoBuiltinsWithOptionalParams["prefixIds"], "prefix"> & {
      /** String or boolean prefix. Function prefixes are not supported in JSON config. */
      prefix?: boolean | string;
    };
    convertColors: Omit<SvgoBuiltinsWithOptionalParams["convertColors"], "currentColor"> & {
      /** Current-color matcher. RegExp values are not supported in JSON config. */
      currentColor?: boolean | string;
    };
    removeComments:
      & Omit<
        SvgoBuiltinsWithOptionalParams["removeComments"],
        "preservePatterns"
      >
      & {
        /** Comment patterns to preserve. Regular expressions should be written as strings. */
        preservePatterns?: readonly string[] | false;
      };
  };

/** Built-in plugins whose params are required in dprint config. */
type DprintBuiltinsWithRequiredParams =
  & Omit<
    SvgoBuiltinsWithRequiredParams,
    "addClassesToSVGElement" | "removeAttributesBySelector"
  >
  & {
    addClassesToSVGElement:
      & Omit<
        SvgoBuiltinsWithRequiredParams["addClassesToSVGElement"],
        "className" | "classNames"
      >
      & {
        /** Single class name to add. Dynamic class callbacks are not supported in JSON config. */
        className?: string;
        /** Multiple class names to add. Dynamic class callbacks are not supported in JSON config. */
        classNames?: string[];
      };
    removeAttributesBySelector: RemoveAttributesBySelectorParams;
  };

/** Built-in plugins that may be referenced by name alone. */
type OptionalPluginConfig = {
  [Name in keyof DprintBuiltinsWithOptionalParams]:
    | Name
    | {
      /** Built-in plugin name. */
      name: Name;
      /** Plugin params. */
      params?: DprintBuiltinsWithOptionalParams[Name];
    };
}[keyof DprintBuiltinsWithOptionalParams];

/** Built-in plugins that require a params object. */
type RequiredPluginConfig = {
  [Name in keyof DprintBuiltinsWithRequiredParams]: {
    /** Built-in plugin name. */
    name: Name;
    /** Plugin params. */
    params: DprintBuiltinsWithRequiredParams[Name];
  };
}[keyof DprintBuiltinsWithRequiredParams];

/** dprint-plugin-svgo configuration. */
export interface DprintPluginSvgoConfig {
  /** Can be used by plugins, for example prefixIds. */
  path?: SvgoConfig["path"];
  /** Pass over SVGs multiple times to ensure all optimizations are applied. */
  multipass?: SvgoConfig["multipass"];
  /** Precision of floating point numbers passed to supporting plugins. */
  floatPrecision?: SvgoConfig["floatPrecision"];
  /** Plugins configuration. Custom JavaScript plugins are not supported in dprint config. */
  plugins?: Array<OptionalPluginConfig | RequiredPluginConfig>;
  /** Options for rendering optimized SVG from AST. */
  js2svg?: DprintStringifyOptions;
  /** Output as Data URI string. */
  datauri?: DataUri;

  /** Number of spaces for indentation in SVG output. */
  indent?: number;
  /** End-of-line character for SVG output. */
  eol?: "lf" | "crlf";
  /** Whether to pretty-print the SVG output. */
  pretty?: boolean;
  /** Whether to add a final newline at the end of the output. */
  finalNewline?: boolean;
  /** Whether to use short self-closing tags. */
  useShortTags?: boolean;

  /** Extension-specific overrides like `svg.multipass`. */
  [key: string]: unknown;
}
