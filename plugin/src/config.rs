use dprint_core::configuration::get_nullable_value;
use dprint_core::configuration::get_value;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigKeyValue;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::ResolveConfigurationResult;
use serde::Serialize;

/// Plugin-specific configuration for SVGO.
#[derive(Clone, Serialize, Default)]
pub struct SvgoPluginConfig {
  // SVGO-specific plugin configuration can be added here
}

/// Configuration for the SVGO plugin.
#[derive(Clone, Serialize, Default)]
pub struct SvgoConfig {
  /// Main configuration options passed to SVGO.
  pub main: serde_json::Map<String, serde_json::Value>,
  /// Extension-specific configuration overrides.
  pub extension_overrides: serde_json::Map<String, serde_json::Value>,
  /// SVGO plugin configuration.
  pub plugins: SvgoPluginConfig,
}

/// Resolves the SVGO configuration from dprint configuration.
///
/// # Arguments
///
/// * `config` - The configuration key map from dprint
/// * `global_config` - Global dprint configuration
///
/// # Returns
///
/// A result containing the resolved `SvgoConfig` and any diagnostics.
#[must_use] pub fn resolve_config(
  mut config: ConfigKeyMap,
  global_config: GlobalConfiguration,
) -> ResolveConfigurationResult<SvgoConfig> {
  let mut diagnostics = Vec::new();
  let mut main: serde_json::Map<String, serde_json::Value> = Default::default();
  let mut extension_overrides: serde_json::Map<String, serde_json::Value> = Default::default();

  let plugins = SvgoPluginConfig {
    // SVGO-specific plugin configuration
  };

  // Handle SVGO js2svg configuration options
  let mut js2svg: serde_json::Map<String, serde_json::Value> = Default::default();

  let dprint_tab_width = get_value(
    &mut config,
    "indentWidth",
    global_config.indent_width.unwrap_or(2),
    &mut diagnostics,
  );
  js2svg.insert(
    "indent".to_string(),
    get_value(&mut config, "indent", dprint_tab_width, &mut diagnostics).into(),
  );

  let dprint_newline_kind: NewLineKind = get_value(
    &mut config,
    "newLineKind",
    global_config.new_line_kind.unwrap_or(NewLineKind::LineFeed),
    &mut diagnostics,
  );
  let eol: Option<String> = get_nullable_value(&mut config, "eol", &mut diagnostics);
  if let Some(eol) = eol {
    js2svg.insert("eol".to_string(), eol.into());
  } else {
    js2svg.insert(
      "eol".to_string(),
      match dprint_newline_kind {
        NewLineKind::CarriageReturnLineFeed => "crlf",
        NewLineKind::LineFeed | NewLineKind::Auto => "lf",
      }
      .into(),
    );
  }

  // Handle other js2svg options
  js2svg.insert(
    "pretty".to_string(),
    get_value(&mut config, "pretty", true, &mut diagnostics).into(),
  );

  main.insert("js2svg".to_string(), serde_json::Value::Object(js2svg));

  // Handle SVGO multipass option
  main.insert(
    "multipass".to_string(),
    get_value(&mut config, "multipass", false, &mut diagnostics).into(),
  );

  for (key, value) in config {
    let mut value = config_key_value_to_json(value);

    // Special handling for plugins key: if it's a string, parse it as JSON
    if key == "plugins" {
      if let serde_json::Value::String(s) = &value {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
          value = parsed;
        }
      }
    }

    if let Some(index) = key.rfind('.') {
      let extension = key[..index].to_lowercase();
      let key = &key[index + 1..];
      extension_overrides
        .entry(extension)
        .or_insert_with(|| serde_json::Value::Object(Default::default()))
        .as_object_mut()
        .unwrap()
        .insert(key.to_string(), value);
    } else {
      main.insert(key, value);
    }
  }

  ResolveConfigurationResult {
    config: SvgoConfig {
      main,
      extension_overrides,
      plugins,
    },
    diagnostics,
  }
}

fn config_key_value_to_json(value: ConfigKeyValue) -> serde_json::Value {
  match value {
    ConfigKeyValue::Bool(value) => value.into(),
    ConfigKeyValue::String(value) => value.into(),
    ConfigKeyValue::Number(value) => value.into(),
    ConfigKeyValue::Object(value) => {
      let mut values = serde_json::Map::new();
      for (key, value) in value {
        values.insert(key, config_key_value_to_json(value));
      }
      serde_json::Value::Object(values)
    }
    ConfigKeyValue::Array(value) => {
      serde_json::Value::Array(value.into_iter().map(config_key_value_to_json).collect())
    }
    ConfigKeyValue::Null => serde_json::Value::Null,
  }
}
