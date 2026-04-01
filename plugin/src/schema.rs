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
  /// Overrides `indent`, `eol`, and `pretty` when set.
  pub js2svg: Option<serde_json::Value>,

  /// Path to the SVG file (used by some SVGO plugins).
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

/// An SVGO plugin entry — either a name string or an object with parameters.
#[derive(JsonSchema, Serialize)]
#[serde(untagged)]
pub enum SvgoPluginEntry {
  /// Plugin name as a string.
  Name(String),
  /// Plugin with configuration.
  WithParams {
    /// Plugin name.
    name: String,
    /// Plugin parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
  },
}
