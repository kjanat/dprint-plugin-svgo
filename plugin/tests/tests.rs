use std::path::PathBuf;
use std::sync::Arc;

use deno_core::futures::FutureExt;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::ConfigKeyValue;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::FormatConfigId;
use dprint_core::plugins::FormatRequest;
use dprint_core::plugins::NullCancellationToken;
use dprint_plugin_deno_base::util::create_tokio_runtime;
use dprint_plugin_svgo::SvgoPluginHandler;

// Handler trait method tests

#[test]
fn plugin_info_returns_correct_values() {
  let handler = SvgoPluginHandler::default();
  let info = handler.plugin_info();

  assert_eq!(info.name, "dprint-plugin-svgo");
  assert_eq!(info.config_key, "svgo");
  assert_eq!(info.help_url, "https://svgo.dev");
  assert!(!info.version.is_empty());
}

#[test]
fn license_text_is_not_empty() {
  let handler = SvgoPluginHandler::default();
  let license = handler.license_text();

  assert!(!license.is_empty());
  assert!(license.contains("MIT"));
}

#[test]
fn resolve_config_returns_svg_extensions() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let result = handler
      .resolve_config(ConfigKeyMap::new(), GlobalConfiguration::default())
      .await;

    assert!(result.diagnostics.is_empty());
    assert!(
      result
        .file_matching
        .file_extensions
        .contains(&"svg".to_string())
    );
  });
}

#[test]
fn resolve_config_with_custom_settings() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let mut config = ConfigKeyMap::new();
    config.insert("multipass".to_string(), ConfigKeyValue::Bool(true));
    config.insert("indent".to_string(), ConfigKeyValue::Number(4));

    let result = handler
      .resolve_config(config, GlobalConfiguration::default())
      .await;

    assert!(result.diagnostics.is_empty());
    // Verify config was resolved
    assert!(
      result
        .config
        .main
        .get("multipass")
        .unwrap()
        .as_bool()
        .unwrap()
    );
  });
}

#[test]
fn format_with_range_returns_none() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("file.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: Some(std::ops::Range { start: 0, end: 10 }),
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Range formatting not supported, should return None
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
  });
}

#[test]
fn format_with_extension_override() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // First resolve config with extension override
    let mut config_map = ConfigKeyMap::new();
    config_map.insert("svg.multipass".to_string(), ConfigKeyValue::Bool(true));

    let resolved = handler
      .resolve_config(config_map, GlobalConfiguration::default())
      .await;

    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" />
</svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(resolved.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
  });
}

#[test]
fn format_with_invalid_utf8_returns_error() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Invalid UTF-8 sequence
    let invalid_bytes = vec![0xff, 0xfe, 0x00, 0x01];

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("file.svg"),
          file_bytes: invalid_bytes,
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should return error for invalid UTF-8
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("UTF-8") || err_msg.contains("utf8"));
  });
}

#[test]
fn format_with_invalid_extension_override() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Create config with invalid extension override (not an object)
    let mut config = dprint_plugin_svgo::config::SvgoConfig::default();
    config.extension_overrides.insert(
      "svg".to_string(),
      serde_json::Value::String("not an object".to_string()),
    );

    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should return error for invalid extension override
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("extension override"));
  });
}

#[test]
fn format_file_without_extension() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("noextension"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should still work - uses main config when no extension
    assert!(result.is_ok());
  });
}

#[test]
fn format_with_multipass_config() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    let mut config_map = ConfigKeyMap::new();
    config_map.insert("multipass".to_string(), ConfigKeyValue::Bool(true));
    config_map.insert("pretty".to_string(), ConfigKeyValue::Bool(true));

    let resolved = handler
      .resolve_config(config_map, GlobalConfiguration::default())
      .await;

    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
      <g>
        <g>
          <rect x="0" y="0" width="100" height="100"/>
        </g>
      </g>
    </svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(resolved.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
    let formatted = result.unwrap();
    assert!(formatted.is_some());
  });
}

#[test]
fn format_preserves_svg_content() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="40" fill="red"/>
</svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
    let formatted = String::from_utf8(result.unwrap().unwrap()).unwrap();
    // Should preserve essential SVG elements
    assert!(formatted.contains("svg"));
    assert!(formatted.contains("circle"));
    assert!(formatted.contains("cx"));
  });
}

// Original integration tests

#[test]
fn handle_invalid_svg() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async move {
    let handler = SvgoPluginHandler::default();
    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("file.svg"),
          file_bytes: b"not valid svg".to_vec(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // SVGO should return None (no change) for invalid SVG rather than error
    assert!(result.is_ok());
  });
}

#[test]
fn handle_valid_svg() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async move {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" />
</svg>"#;
    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("file.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
    // SVGO should optimize the SVG
    let formatted = result.unwrap();
    assert!(formatted.is_some());
  });
}
