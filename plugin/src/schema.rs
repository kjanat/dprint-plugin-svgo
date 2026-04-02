use schemars::JsonSchema;
use serde::Serialize;

/// Configuration for the dprint-plugin-svgo plugin.
///
/// All fields are optional. Options not set here inherit from the
/// dprint global configuration or SVGO defaults.
#[derive(JsonSchema, Serialize)]
#[schemars(title = "dprint-plugin-svgo configuration")]
pub struct SvgoConfigSchema {
  /// Number of spaces for indentation in SVG output.
  /// Inherited from dprint global `indentWidth` when unset.
  #[schemars(range(min = 0))]
  pub indent: Option<u32>,

  /// End-of-line character for SVG output.
  /// Inherited from dprint global `newLineKind` when unset.
  pub eol: Option<EolConfig>,

  /// Whether to pretty-print the SVG output.
  pub pretty: Option<bool>,

  /// Whether to add a final newline at the end of the output.
  #[serde(rename = "finalNewline")]
  pub final_newline: Option<bool>,

  /// Whether to use short self-closing tags (e.g. `<path/>` instead of `<path></path>`).
  #[serde(rename = "useShortTags")]
  pub use_short_tags: Option<bool>,

  /// Whether to enable multipass optimization.
  /// Multiple passes may produce a smaller output.
  pub multipass: Option<bool>,

  /// Array of SVGO plugin configurations.
  pub plugins: Option<Vec<SvgoPluginEntry>>,

  /// Number of digits after the decimal point (0–20).
  #[serde(rename = "floatPrecision")]
  #[schemars(range(min = 0, max = 20))]
  pub float_precision: Option<u32>,

  /// Type of Data URI encoding.
  pub datauri: Option<DataUriConfig>,

  /// Direct js2svg configuration object passed to SVGO.
  /// Overrides `indent`, `eol`, `pretty`, `finalNewline`, and `useShortTags` when set.
  pub js2svg: Option<Js2SvgOptions>,

  /// Path to the SVG file (used by some SVGO plugins like `prefixIds`).
  pub path: Option<String>,
}

/// End-of-line character.
#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EolConfig {
  /// Unix-style line feed.
  Lf,
  /// Windows-style carriage return + line feed.
  Crlf,
}

/// Data URI encoding mode.
#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataUriConfig {
  /// Base64 encoding.
  Base64,
  /// URL-safe encoding.
  Enc,
  /// Unencoded.
  Unenc,
}

/// SVG serialization options (js2svg).
#[derive(JsonSchema, Serialize)]
pub struct Js2SvgOptions {
  /// Number of spaces for indentation.
  pub indent: Option<u32>,
  /// End-of-line character (`"lf"` or `"crlf"`).
  pub eol: Option<EolConfig>,
  /// Whether to pretty-print the output.
  pub pretty: Option<bool>,
  /// Whether to add a final newline at the end of the output.
  #[serde(rename = "finalNewline")]
  pub final_newline: Option<bool>,
  /// Whether to use short self-closing tags.
  #[serde(rename = "useShortTags")]
  pub use_short_tags: Option<bool>,
}

/// An SVGO plugin entry — either a plugin name, a preset, or a plugin with parameters.
#[derive(JsonSchema, Serialize)]
#[serde(untagged)]
pub enum SvgoPluginEntry {
  /// Plugin referenced by name (e.g. `"removeComments"`).
  Name(SvgoBuiltinPlugin),
  /// Preset or plugin with configuration.
  WithParams {
    /// Plugin or preset name.
    name: SvgoPluginName,
    /// Plugin parameters (structure depends on the plugin).
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
  },
}

/// All SVGO plugin and preset names.
#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SvgoPluginName {
  /// Preset with the default set of plugins.
  #[serde(rename = "preset-default")]
  PresetDefault,
  /// All individual built-in plugins.
  #[serde(untagged)]
  Plugin(SvgoBuiltinPlugin),
}

/// All SVGO built-in plugin IDs.
#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SvgoBuiltinPlugin {
  // --- Included in preset-default ---
  /// Remove `<!DOCTYPE>` declaration.
  RemoveDoctype,
  /// Remove XML processing instructions.
  #[serde(rename = "removeXMLProcInst")]
  RemoveXmlProcInst,
  /// Remove comments.
  RemoveComments,
  /// Remove `<metadata>`.
  RemoveMetadata,
  /// Remove editors' namespace data.
  #[serde(rename = "removeEditorsNSData")]
  RemoveEditorsNsData,
  /// Clean up attribute whitespace.
  CleanupAttrs,
  /// Merge multiple `<style>` elements.
  MergeStyles,
  /// Inline styles into attributes where possible.
  InlineStyles,
  /// Minify `<style>` content.
  MinifyStyles,
  /// Clean up `id` attributes and references.
  CleanupIds,
  /// Remove unused `<defs>`.
  RemoveUselessDefs,
  /// Round numeric values and remove defaults.
  CleanupNumericValues,
  /// Convert color values to shorter forms.
  ConvertColors,
  /// Remove unknown elements and attributes, reset defaults.
  RemoveUnknownsAndDefaults,
  /// Remove non-inheritable group attributes.
  RemoveNonInheritableGroupAttrs,
  /// Remove useless `stroke` and `fill` attributes.
  RemoveUselessStrokeAndFill,
  /// Remove `enable-background` attribute.
  CleanupEnableBackground,
  /// Remove hidden elements.
  RemoveHiddenElems,
  /// Remove empty text elements.
  RemoveEmptyText,
  /// Convert basic shapes to `<path>`.
  ConvertShapeToPath,
  /// Convert `<ellipse>` with equal radii to `<circle>`.
  ConvertEllipseToCircle,
  /// Move element attributes to enclosing group.
  MoveElemsAttrsToGroup,
  /// Move group attributes to elements.
  MoveGroupAttrsToElems,
  /// Collapse useless groups.
  CollapseGroups,
  /// Optimize path data.
  ConvertPathData,
  /// Optimize transform attribute values.
  ConvertTransform,
  /// Remove empty attributes.
  RemoveEmptyAttrs,
  /// Remove empty container elements.
  RemoveEmptyContainers,
  /// Remove unused namespace declarations.
  #[serde(rename = "removeUnusedNS")]
  RemoveUnusedNs,
  /// Merge multiple paths into one.
  MergePaths,
  /// Sort element attributes.
  SortAttrs,
  /// Sort children of `<defs>`.
  SortDefsChildren,
  /// Remove `<desc>`.
  RemoveDesc,

  // --- Not in preset-default ---
  /// Add attributes to `<svg>`.
  #[serde(rename = "addAttributesToSVGElement")]
  AddAttributesToSvgElement,
  /// Add classes to `<svg>`.
  #[serde(rename = "addClassesToSVGElement")]
  AddClassesToSvgElement,
  /// Clean up list of values (e.g. `viewBox`, `points`).
  CleanupListOfValues,
  /// Convert one-stop gradients to solid colors.
  ConvertOneStopGradients,
  /// Convert `style` attributes to presentation attributes.
  ConvertStyleToAttrs,
  /// Prefix `id` and class names to avoid collisions.
  PrefixIds,
  /// Remove attributes by CSS selector.
  RemoveAttributesBySelector,
  /// Remove specified attributes.
  RemoveAttrs,
  /// Remove deprecated attributes.
  RemoveDeprecatedAttrs,
  /// Remove `width` and `height` in favor of `viewBox`.
  RemoveDimensions,
  /// Remove elements by `id` or CSS selector.
  RemoveElementsByAttr,
  /// Remove off-canvas paths.
  RemoveOffCanvasPaths,
  /// Remove raster image elements.
  RemoveRasterImages,
  /// Remove `<script>` elements.
  RemoveScripts,
  /// Remove `<style>` elements.
  RemoveStyleElement,
  /// Remove `<title>`.
  RemoveTitle,
  /// Remove `viewBox` attribute.
  RemoveViewBox,
  /// Remove `xmlns` attribute (for inline SVG).
  #[serde(rename = "removeXMLNS")]
  RemoveXmlns,
  /// Remove deprecated `xlink:` attributes.
  RemoveXlink,
  /// Reuse equal `<path>` elements via `<use>`.
  ReusePaths,
}
