use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigKeyValue;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::NewLineKind;
use dprint_plugin_svgo::config::resolve_config;
use proptest::prelude::*;

fn empty_global_config() -> GlobalConfiguration {
  GlobalConfiguration {
    line_width: None,
    use_tabs: None,
    indent_width: None,
    new_line_kind: None,
  }
}

#[test]
fn resolve_config_defaults() {
  let config = ConfigKeyMap::new();
  let result = resolve_config(config, empty_global_config());

  assert!(result.diagnostics.is_empty());

  // Check default js2svg settings using accessor methods
  assert_eq!(result.config.get_indent(), Some(2));
  assert_eq!(result.config.get_eol(), Some("lf"));
  assert_eq!(result.config.is_pretty(), Some(true));

  // Check default multipass
  assert_eq!(result.config.is_multipass(), Some(false));
}

#[test]
fn resolve_config_with_custom_indent() {
  let mut config = ConfigKeyMap::new();
  config.insert("indent".to_string(), ConfigKeyValue::Number(4));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.config.get_indent(), Some(4));
}

#[test]
fn resolve_config_with_global_indent_width() {
  let config = ConfigKeyMap::new();
  let global_config = GlobalConfiguration {
    indent_width: Some(8),
    ..empty_global_config()
  };

  let result = resolve_config(config, global_config);

  assert_eq!(result.config.get_indent(), Some(8));
}

#[test]
fn resolve_config_local_overrides_global() {
  let mut config = ConfigKeyMap::new();
  config.insert("indent".to_string(), ConfigKeyValue::Number(3));
  let global_config = GlobalConfiguration {
    indent_width: Some(8),
    ..empty_global_config()
  };

  let result = resolve_config(config, global_config);

  // Local indent (3) should override global indent_width (8)
  assert_eq!(result.config.get_indent(), Some(3));
}

#[test]
fn resolve_config_with_crlf_eol() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "eol".to_string(),
    ConfigKeyValue::String("crlf".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.config.get_eol(), Some("crlf"));
}

#[test]
fn resolve_config_with_global_newline_kind() {
  let config = ConfigKeyMap::new();
  let global_config = GlobalConfiguration {
    new_line_kind: Some(NewLineKind::CarriageReturnLineFeed),
    ..empty_global_config()
  };

  let result = resolve_config(config, global_config);

  assert_eq!(result.config.get_eol(), Some("crlf"));
}

#[test]
fn resolve_config_with_multipass() {
  let mut config = ConfigKeyMap::new();
  config.insert("multipass".to_string(), ConfigKeyValue::Bool(true));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.config.is_multipass(), Some(true));
}

#[test]
fn resolve_config_with_pretty_false() {
  let mut config = ConfigKeyMap::new();
  config.insert("pretty".to_string(), ConfigKeyValue::Bool(false));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.config.is_pretty(), Some(false));
}

#[test]
fn resolve_config_with_extension_override() {
  let mut config = ConfigKeyMap::new();
  config.insert("svg.multipass".to_string(), ConfigKeyValue::Bool(true));

  let result = resolve_config(config, empty_global_config());

  // Main config should have default multipass
  assert_eq!(result.config.is_multipass(), Some(false));

  // Extension override should have multipass true
  let svg_override = result
    .config
    .get_extension_override("svg")
    .unwrap()
    .as_object()
    .unwrap();
  assert!(svg_override.get("multipass").unwrap().as_bool().unwrap());
}

#[test]
fn resolve_config_with_multiple_extension_overrides() {
  let mut config = ConfigKeyMap::new();
  config.insert("svg.multipass".to_string(), ConfigKeyValue::Bool(true));
  config.insert("svg.pretty".to_string(), ConfigKeyValue::Bool(false));
  config.insert("svgz.multipass".to_string(), ConfigKeyValue::Bool(false));

  let result = resolve_config(config, empty_global_config());

  // SVG override
  let svg_override = result
    .config
    .get_extension_override("svg")
    .unwrap()
    .as_object()
    .unwrap();
  assert!(svg_override.get("multipass").unwrap().as_bool().unwrap());
  assert!(!svg_override.get("pretty").unwrap().as_bool().unwrap());

  // SVGZ override
  let svgz_override = result
    .config
    .get_extension_override("svgz")
    .unwrap()
    .as_object()
    .unwrap();
  assert!(!svgz_override.get("multipass").unwrap().as_bool().unwrap());
}

#[test]
fn resolve_config_plugins_as_string_json() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "plugins".to_string(),
    ConfigKeyValue::String(r#"["preset-default"]"#.to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  // The string should be parsed as JSON array
  let plugins = result.config.main.get("plugins").unwrap();
  assert!(plugins.is_array());
  let arr = plugins.as_array().unwrap();
  assert_eq!(arr.len(), 1);
  assert_eq!(arr[0].as_str().unwrap(), "preset-default");
}

#[test]
fn resolve_config_plugins_as_array() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "plugins".to_string(),
    ConfigKeyValue::Array(vec![ConfigKeyValue::String("preset-default".to_string())]),
  );

  let result = resolve_config(config, empty_global_config());

  let plugins = result.config.main.get("plugins").unwrap();
  assert!(plugins.is_array());
}

#[test]
fn resolve_config_unknown_key_passes_through_with_warning() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "customOption".to_string(),
    ConfigKeyValue::String("value".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  // Unknown keys pass through but generate a diagnostic warning
  assert_eq!(
    result
      .config
      .main
      .get("customOption")
      .unwrap()
      .as_str()
      .unwrap(),
    "value"
  );

  // Should warn about unknown key
  assert_eq!(result.diagnostics.len(), 1);
  assert!(
    result.diagnostics[0]
      .message
      .contains("Unknown SVGO option")
  );
}

#[test]
fn resolve_config_extension_case_insensitive() {
  let mut config = ConfigKeyMap::new();
  config.insert("SVG.multipass".to_string(), ConfigKeyValue::Bool(true));

  let result = resolve_config(config, empty_global_config());

  // Should be stored as lowercase
  assert!(result.config.has_extension_override("svg"));
  assert!(!result.config.has_extension_override("SVG"));
}

// Tests for config_key_value_to_json recursive cases

#[test]
fn resolve_config_with_nested_object() {
  let mut config = ConfigKeyMap::new();
  let mut nested = ConfigKeyMap::new();
  nested.insert(
    "innerKey".to_string(),
    ConfigKeyValue::String("innerValue".to_string()),
  );
  nested.insert("innerNum".to_string(), ConfigKeyValue::Number(42));
  config.insert("customObject".to_string(), ConfigKeyValue::Object(nested));

  let result = resolve_config(config, empty_global_config());

  let obj = result
    .config
    .main
    .get("customObject")
    .unwrap()
    .as_object()
    .unwrap();
  assert_eq!(obj.get("innerKey").unwrap().as_str().unwrap(), "innerValue");
  assert_eq!(obj.get("innerNum").unwrap().as_i64().unwrap(), 42);
}

#[test]
fn resolve_config_with_array_of_mixed_types() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "mixedArray".to_string(),
    ConfigKeyValue::Array(vec![
      ConfigKeyValue::String("str".to_string()),
      ConfigKeyValue::Number(123),
      ConfigKeyValue::Bool(true),
      ConfigKeyValue::Null,
    ]),
  );

  let result = resolve_config(config, empty_global_config());

  let arr = result
    .config
    .main
    .get("mixedArray")
    .unwrap()
    .as_array()
    .unwrap();
  assert_eq!(arr.len(), 4);
  assert_eq!(arr[0].as_str().unwrap(), "str");
  assert_eq!(arr[1].as_i64().unwrap(), 123);
  assert!(arr[2].as_bool().unwrap());
  assert!(arr[3].is_null());
}

#[test]
fn resolve_config_with_deeply_nested_object() {
  let mut config = ConfigKeyMap::new();

  let mut level2 = ConfigKeyMap::new();
  level2.insert(
    "deep".to_string(),
    ConfigKeyValue::String("value".to_string()),
  );

  let mut level1 = ConfigKeyMap::new();
  level1.insert("nested".to_string(), ConfigKeyValue::Object(level2));

  config.insert("outer".to_string(), ConfigKeyValue::Object(level1));

  let result = resolve_config(config, empty_global_config());

  let outer = result
    .config
    .main
    .get("outer")
    .unwrap()
    .as_object()
    .unwrap();
  let nested = outer.get("nested").unwrap().as_object().unwrap();
  assert_eq!(nested.get("deep").unwrap().as_str().unwrap(), "value");
}

#[test]
fn resolve_config_with_array_of_objects() {
  let mut config = ConfigKeyMap::new();

  let mut obj1 = ConfigKeyMap::new();
  obj1.insert(
    "name".to_string(),
    ConfigKeyValue::String("first".to_string()),
  );

  let mut obj2 = ConfigKeyMap::new();
  obj2.insert(
    "name".to_string(),
    ConfigKeyValue::String("second".to_string()),
  );

  config.insert(
    "objectArray".to_string(),
    ConfigKeyValue::Array(vec![
      ConfigKeyValue::Object(obj1),
      ConfigKeyValue::Object(obj2),
    ]),
  );

  let result = resolve_config(config, empty_global_config());

  let arr = result
    .config
    .main
    .get("objectArray")
    .unwrap()
    .as_array()
    .unwrap();
  assert_eq!(arr.len(), 2);
  assert_eq!(
    arr[0]
      .as_object()
      .unwrap()
      .get("name")
      .unwrap()
      .as_str()
      .unwrap(),
    "first"
  );
  assert_eq!(
    arr[1]
      .as_object()
      .unwrap()
      .get("name")
      .unwrap()
      .as_str()
      .unwrap(),
    "second"
  );
}

#[test]
fn resolve_config_with_null_value() {
  let mut config = ConfigKeyMap::new();
  config.insert("nullKey".to_string(), ConfigKeyValue::Null);

  let result = resolve_config(config, empty_global_config());

  assert!(result.config.main.get("nullKey").unwrap().is_null());
}

#[test]
fn resolve_config_plugins_invalid_json_string_kept_as_string() {
  let mut config = ConfigKeyMap::new();
  // Invalid JSON string should remain as string
  config.insert(
    "plugins".to_string(),
    ConfigKeyValue::String("not valid json [".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  // Should remain as string since JSON parsing failed
  assert_eq!(
    result.config.main.get("plugins").unwrap().as_str().unwrap(),
    "not valid json ["
  );
}

#[test]
fn resolve_config_empty_extension_override() {
  let mut config = ConfigKeyMap::new();
  // Extension with no actual override key (edge case)
  config.insert("svg.".to_string(), ConfigKeyValue::Bool(true));

  let result = resolve_config(config, empty_global_config());

  // Should create svg extension with empty key
  let svg_override = result
    .config
    .extension_overrides
    .get("svg")
    .unwrap()
    .as_object()
    .unwrap();
  assert!(svg_override.contains_key(""));
}

#[test]
fn resolve_config_multiple_dots_in_key() {
  let mut config = ConfigKeyMap::new();
  // Key with multiple dots - only last dot should be used for extension split
  config.insert(
    "some.extension.key".to_string(),
    ConfigKeyValue::String("value".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  // Should split on last dot: extension = "some.extension", key = "key"
  let override_config = result
    .config
    .extension_overrides
    .get("some.extension")
    .unwrap()
    .as_object()
    .unwrap();
  assert_eq!(
    override_config.get("key").unwrap().as_str().unwrap(),
    "value"
  );
}

#[test]
fn resolve_config_all_js2svg_options() {
  let mut config = ConfigKeyMap::new();
  config.insert("indent".to_string(), ConfigKeyValue::Number(4));
  config.insert(
    "eol".to_string(),
    ConfigKeyValue::String("crlf".to_string()),
  );
  config.insert("pretty".to_string(), ConfigKeyValue::Bool(false));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.config.get_indent(), Some(4));
  assert_eq!(result.config.get_eol(), Some("crlf"));
  assert_eq!(result.config.is_pretty(), Some(false));
}

#[test]
fn resolve_config_global_auto_newline() {
  let config = ConfigKeyMap::new();
  let global_config = GlobalConfiguration {
    new_line_kind: Some(NewLineKind::Auto),
    ..empty_global_config()
  };

  let result = resolve_config(config, global_config);

  // Auto should default to lf
  assert_eq!(result.config.get_eol(), Some("lf"));
}

// Schema validation tests

#[test]
fn resolve_config_invalid_plugins_not_array() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "plugins".to_string(),
    ConfigKeyValue::String("preset-default".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.diagnostics.len(), 1);
  assert!(result.diagnostics[0].message.contains("array"));
}

#[test]
fn resolve_config_invalid_float_precision_negative() {
  let mut config = ConfigKeyMap::new();
  config.insert("floatPrecision".to_string(), ConfigKeyValue::Number(-1));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.diagnostics.len(), 1);
  assert!(result.diagnostics[0].message.contains("between 0 and 20"));
}

#[test]
fn resolve_config_invalid_float_precision_too_high() {
  let mut config = ConfigKeyMap::new();
  config.insert("floatPrecision".to_string(), ConfigKeyValue::Number(25));

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.diagnostics.len(), 1);
  assert!(result.diagnostics[0].message.contains("between 0 and 20"));
}

#[test]
fn resolve_config_invalid_datauri() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "datauri".to_string(),
    ConfigKeyValue::String("invalid".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.diagnostics.len(), 1);
  assert!(result.diagnostics[0].message.contains("base64"));
}

#[test]
fn resolve_config_invalid_js2svg_not_object() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "js2svg".to_string(),
    ConfigKeyValue::String("invalid".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  assert_eq!(result.diagnostics.len(), 1);
  assert!(result.diagnostics[0].message.contains("object"));
}

#[test]
fn resolve_config_valid_plugins_array() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "plugins".to_string(),
    ConfigKeyValue::Array(vec![ConfigKeyValue::String("preset-default".to_string())]),
  );

  let result = resolve_config(config, empty_global_config());

  // No diagnostics for valid array
  assert!(result.diagnostics.is_empty());
}

#[test]
fn resolve_config_valid_datauri() {
  let mut config = ConfigKeyMap::new();
  config.insert(
    "datauri".to_string(),
    ConfigKeyValue::String("base64".to_string()),
  );

  let result = resolve_config(config, empty_global_config());

  assert!(result.diagnostics.is_empty());
}

#[test]
fn resolve_config_valid_float_precision() {
  let mut config = ConfigKeyMap::new();
  config.insert("floatPrecision".to_string(), ConfigKeyValue::Number(5));

  let result = resolve_config(config, empty_global_config());

  assert!(result.diagnostics.is_empty());
}

// Tests for SvgoConfig accessor methods

#[test]
fn svgo_config_get_main_value() {
  let mut config = ConfigKeyMap::new();
  config.insert("multipass".to_string(), ConfigKeyValue::Bool(true));
  config.insert("floatPrecision".to_string(), ConfigKeyValue::Number(3));

  let result = resolve_config(config, empty_global_config());

  // Test get_main_value
  assert!(result.config.get_main_value("multipass").is_some());
  assert!(result.config.get_main_value("js2svg").is_some());
  assert!(result.config.get_main_value("nonexistent").is_none());
}

#[test]
fn svgo_config_get_js2svg() {
  let config = ConfigKeyMap::new();
  let result = resolve_config(config, empty_global_config());

  // Test get_js2svg returns the js2svg object
  let js2svg = result.config.get_js2svg();
  assert!(js2svg.is_some());

  let js2svg = js2svg.unwrap();
  assert!(js2svg.contains_key("indent"));
  assert!(js2svg.contains_key("eol"));
  assert!(js2svg.contains_key("pretty"));
}

#[test]
fn svgo_config_has_extension_override() {
  let mut config = ConfigKeyMap::new();
  config.insert("svg.multipass".to_string(), ConfigKeyValue::Bool(true));

  let result = resolve_config(config, empty_global_config());

  // Test has_extension_override
  assert!(result.config.has_extension_override("svg"));
  assert!(!result.config.has_extension_override("svgz"));
  assert!(!result.config.has_extension_override("png"));
}

// Property-based tests

proptest! {
  #[test]
  fn prop_resolve_config_doesnt_panic_with_numeric_indent(indent in -1000i32..1000) {
    let mut config = ConfigKeyMap::new();
    config.insert("indent".to_string(), ConfigKeyValue::Number(indent));
    let _ = resolve_config(config, empty_global_config());
  }

  #[test]
  fn prop_resolve_config_doesnt_panic_with_float_precision(precision in -100i32..100) {
    let mut config = ConfigKeyMap::new();
    config.insert("floatPrecision".to_string(), ConfigKeyValue::Number(precision));
    let _ = resolve_config(config, empty_global_config());
  }

  #[test]
  fn prop_resolve_config_doesnt_panic_with_arbitrary_string_values(value in "\\PC*") {
    let mut config = ConfigKeyMap::new();
    config.insert("eol".to_string(), ConfigKeyValue::String(value));
    let _ = resolve_config(config, empty_global_config());
  }

  #[test]
  fn prop_resolve_config_handles_arbitrary_extension_keys(ext in "[a-z]{1,10}", key in "[a-zA-Z]{1,20}") {
    let mut config = ConfigKeyMap::new();
    config.insert(format!("{ext}.{key}"), ConfigKeyValue::Bool(true));
    let _ = resolve_config(config, empty_global_config());
  }

  #[test]
  fn prop_resolve_config_handles_combined_settings(
    indent in 0i32..20,
    multipass in prop::bool::ANY,
    pretty in prop::bool::ANY,
  ) {
    let mut config = ConfigKeyMap::new();
    config.insert("indent".to_string(), ConfigKeyValue::Number(indent));
    config.insert("multipass".to_string(), ConfigKeyValue::Bool(multipass));
    config.insert("pretty".to_string(), ConfigKeyValue::Bool(pretty));

    let result = resolve_config(config, empty_global_config());

    // Should produce valid config with expected values
    assert_eq!(result.config.get_indent(), Some(i64::from(indent)));
    assert_eq!(result.config.is_multipass(), Some(multipass));
    assert_eq!(result.config.is_pretty(), Some(pretty));
  }

  #[test]
  fn prop_resolve_config_handles_global_config_variations(
    indent_width in prop::option::of(1u8..20),
    use_tabs in prop::option::of(prop::bool::ANY),
  ) {
    let config = ConfigKeyMap::new();
    let global_config = GlobalConfiguration {
      indent_width,
      use_tabs,
      line_width: None,
      new_line_kind: None,
    };

    let result = resolve_config(config, global_config);

    // Should use global indent_width if set, otherwise default to 2
    let expected_indent = i64::from(indent_width.unwrap_or(2));
    assert_eq!(result.config.get_indent(), Some(expected_indent));
  }
}
