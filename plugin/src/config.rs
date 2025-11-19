use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigKeyValue;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::ResolveConfigurationResult;
use dprint_core::configuration::get_nullable_value;
use dprint_core::configuration::get_value;
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

#[allow(dead_code)] // Public API methods for library consumers
impl SvgoConfig {
  /// Get the js2svg configuration object.
  #[must_use]
  pub fn get_js2svg(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
    self.main.get("js2svg").and_then(|v| v.as_object())
  }

  /// Get the configured indent value.
  #[must_use]
  pub fn get_indent(&self) -> Option<i64> {
    self
      .get_js2svg()
      .and_then(|js2svg| js2svg.get("indent"))
      .and_then(|v| v.as_i64())
  }

  /// Get the configured end-of-line style.
  #[must_use]
  pub fn get_eol(&self) -> Option<&str> {
    self
      .get_js2svg()
      .and_then(|js2svg| js2svg.get("eol"))
      .and_then(|v| v.as_str())
  }

  /// Get whether pretty printing is enabled.
  #[must_use]
  pub fn is_pretty(&self) -> Option<bool> {
    self
      .get_js2svg()
      .and_then(|js2svg| js2svg.get("pretty"))
      .and_then(|v| v.as_bool())
  }

  /// Get whether multipass optimization is enabled.
  #[must_use]
  pub fn is_multipass(&self) -> Option<bool> {
    self.main.get("multipass").and_then(|v| v.as_bool())
  }

  /// Get a value from the main configuration.
  #[must_use]
  pub fn get_main_value(&self, key: &str) -> Option<&serde_json::Value> {
    self.main.get(key)
  }

  /// Get extension-specific override configuration.
  #[must_use]
  pub fn get_extension_override(&self, ext: &str) -> Option<&serde_json::Value> {
    self.extension_overrides.get(ext)
  }

  /// Check if an extension has override configuration.
  #[must_use]
  pub fn has_extension_override(&self, ext: &str) -> bool {
    self.extension_overrides.contains_key(ext)
  }
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
#[must_use]
pub fn resolve_config(
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
    if key == "plugins"
      && let serde_json::Value::String(s) = &value
      && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s)
    {
      value = parsed;
    }

    // Validate configuration schema
    validate_config_value(&key, &value, &mut diagnostics);

    if let Some(index) = key.rfind('.') {
      let extension = key[..index].to_lowercase();
      let key = &key[index + 1..];
      let entry = extension_overrides
        .entry(extension)
        .or_insert_with(|| serde_json::Value::Object(Default::default()));
      // Safe: we just inserted an Object above if missing
      if let Some(obj) = entry.as_object_mut() {
        obj.insert(key.to_string(), value);
      }
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

/// Validates a configuration value and adds diagnostics for invalid values.
fn validate_config_value(
  key: &str,
  value: &serde_json::Value,
  diagnostics: &mut Vec<dprint_core::configuration::ConfigurationDiagnostic>,
) {
  use dprint_core::configuration::ConfigurationDiagnostic;

  // Extract the base key (without extension prefix like "svg.")
  let base_key = key.rfind('.').map_or(key, |i| &key[i + 1..]);

  match base_key {
    "plugins" => {
      if !value.is_array() {
        diagnostics.push(ConfigurationDiagnostic {
          property_name: key.to_string(),
          message: "Expected 'plugins' to be an array of plugin configurations".to_string(),
        });
      }
    }
    "floatPrecision" => {
      if let Some(n) = value.as_i64() {
        if !(0..=20).contains(&n) {
          diagnostics.push(ConfigurationDiagnostic {
            property_name: key.to_string(),
            message: format!("'floatPrecision' should be between 0 and 20, got {n}"),
          });
        }
      } else if !value.is_number() {
        diagnostics.push(ConfigurationDiagnostic {
          property_name: key.to_string(),
          message: "Expected 'floatPrecision' to be a number".to_string(),
        });
      }
    }
    "datauri" => {
      if let Some(s) = value.as_str() {
        if !["base64", "enc", "unenc"].contains(&s) {
          diagnostics.push(ConfigurationDiagnostic {
            property_name: key.to_string(),
            message: format!("'datauri' must be 'base64', 'enc', or 'unenc', got '{s}'"),
          });
        }
      } else {
        diagnostics.push(ConfigurationDiagnostic {
          property_name: key.to_string(),
          message: "Expected 'datauri' to be a string".to_string(),
        });
      }
    }
    "js2svg" => {
      if !value.is_object() {
        diagnostics.push(ConfigurationDiagnostic {
          property_name: key.to_string(),
          message: "Expected 'js2svg' to be an object".to_string(),
        });
      }
    }
    _ => {
      // No validation for unknown keys - SVGO may accept additional options
    }
  }
}
