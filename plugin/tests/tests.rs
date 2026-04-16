use std::path::PathBuf;
use std::sync::Arc;

use deno_core::futures::FutureExt;
use deno_core::futures::future::join_all;
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
    config.insert("finalNewline".to_string(), ConfigKeyValue::Bool(true));
    config.insert("indent".to_string(), ConfigKeyValue::Number(4));

    let result = handler
      .resolve_config(config, GlobalConfiguration::default())
      .await;

    assert!(result.diagnostics.is_empty());
    // Verify config was resolved
    let js2svg = result
      .config
      .main
      .get("js2svg")
      .unwrap()
      .as_object()
      .unwrap();
    assert!(js2svg.get("finalNewline").unwrap().as_bool().unwrap());
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
    config_map.insert("svg.pretty".to_string(), ConfigKeyValue::Bool(false));

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
fn format_with_website_example_plugins() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    let mut prefix_params = ConfigKeyMap::new();
    prefix_params.insert(
      "prefix".to_string(),
      ConfigKeyValue::String("icon".to_string()),
    );

    let mut prefix_plugin = ConfigKeyMap::new();
    prefix_plugin.insert(
      "name".to_string(),
      ConfigKeyValue::String("prefixIds".to_string()),
    );
    prefix_plugin.insert("params".to_string(), ConfigKeyValue::Object(prefix_params));

    let mut config_map = ConfigKeyMap::new();
    config_map.insert("pretty".to_string(), ConfigKeyValue::Bool(true));
    config_map.insert("indent".to_string(), ConfigKeyValue::Number(2));
    config_map.insert(
      "plugins".to_string(),
      ConfigKeyValue::Array(vec![
        ConfigKeyValue::String("preset-default".to_string()),
        ConfigKeyValue::String("removeViewBox".to_string()),
        ConfigKeyValue::Object(prefix_plugin),
      ]),
    );

    let resolved = handler
      .resolve_config(config_map, GlobalConfiguration::default())
      .await;
    assert!(resolved.diagnostics.is_empty());

    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10">
  <defs>
    <clipPath id="shape">
      <path d="M0 0h10v10H0z" />
    </clipPath>
  </defs>
  <g clip-path="url(#shape)">
    <path d="M0 0h5v5H0z" />
  </g>
</svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("website-example.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(resolved.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
    let output = String::from_utf8(result.unwrap().unwrap()).unwrap();
    assert!(output.contains("id=\"icon__a\""));
    assert!(output.contains("clip-path=\"url(#icon__a)\""));
    assert!(!output.contains("viewBox"));
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
fn format_with_final_newline_config() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    let mut config_map = ConfigKeyMap::new();
    config_map.insert("pretty".to_string(), ConfigKeyValue::Bool(true));
    config_map.insert("finalNewline".to_string(), ConfigKeyValue::Bool(true));

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
    let formatted = String::from_utf8(result.unwrap().unwrap()).unwrap();
    // Should still be valid SVG.
    assert!(formatted.contains("svg"));
    assert!(formatted.ends_with('\n'));
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

#[test]
fn extension_override_affects_output() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG fixture used to verify extension-specific overrides.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
      <g>
        <g>
          <rect x="0" y="0" width="100" height="100"/>
        </g>
      </g>
    </svg>"#;

    // Format with default pretty output.
    let default_config = handler
      .resolve_config(ConfigKeyMap::new(), GlobalConfiguration::default())
      .await;

    let default_result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(default_config.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Format with per-extension compact output.
    let mut config_map = ConfigKeyMap::new();
    config_map.insert("svg.pretty".to_string(), ConfigKeyValue::Bool(false));

    let compact_config = handler
      .resolve_config(config_map, GlobalConfiguration::default())
      .await;

    let compact_result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(compact_config.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Both should succeed
    assert!(default_result.is_ok());
    assert!(compact_result.is_ok());

    let default_output = default_result.unwrap();
    let compact_output = compact_result.unwrap();

    // Both should produce output
    assert!(default_output.is_some());
    assert!(compact_output.is_some());

    // Compact output should be no larger than pretty output.
    let len_default = default_output.unwrap().len();
    let len_compact = compact_output.unwrap().len();

    assert!(
      len_compact <= len_default,
      "Compact output ({}) should not be larger than default ({})",
      len_compact,
      len_default
    );
  });
}

// Concurrent formatting tests

#[test]
fn concurrent_formatting_multiple_files() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let mut futures = Vec::new();

    // Create 5 concurrent format futures
    for i in 0..5 {
      let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" id="svg{}">
  <rect x="0" y="0" width="{}" height="100"/>
</svg>"#,
        i,
        i * 10 + 10
      );

      futures.push(handler.format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(i as u32),
          file_path: PathBuf::from(format!("file{}.svg", i)),
          file_bytes: svg.into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      ));
    }

    // Execute all concurrently
    let results = join_all(futures).await;

    // All tasks should complete successfully
    assert_eq!(results.len(), 5);
    for result in results {
      assert!(result.is_ok(), "Format failed");
    }
  });
}

// Empty/minimal SVG edge case tests

#[test]
fn format_empty_string() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("empty.svg"),
          file_bytes: vec![],
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Empty input should not cause panic
    assert!(result.is_ok());
  });
}

#[test]
fn format_minimal_svg() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg></svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("minimal.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
  });
}

#[test]
fn format_whitespace_only() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = "   \n\t  \n   ";

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("whitespace.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Whitespace-only input should not cause panic
    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_namespace_only() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("namespace.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Minimal SVG may return None if there's nothing to optimize
    assert!(result.is_ok());
  });
}

// Config diagnostic tests

#[test]
fn resolve_config_with_wrong_type_indent() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let mut config = ConfigKeyMap::new();
    // String instead of number for indent
    config.insert(
      "indent".to_string(),
      ConfigKeyValue::String("four".to_string()),
    );

    let result = handler
      .resolve_config(config, GlobalConfiguration::default())
      .await;

    // String "four" can't be parsed as number, generates diagnostic
    assert_eq!(result.diagnostics.len(), 1);
    assert!(result.diagnostics[0].message.contains("invalid digit"));

    // Config still resolves with default indent value
    let js2svg = result
      .config
      .main
      .get("js2svg")
      .expect("js2svg should exist")
      .as_object()
      .expect("js2svg should be object");
    let indent = js2svg.get("indent").expect("indent should exist");
    // Falls back to default when parsing fails
    assert!(indent.is_number(), "Should use default number indent");
  });
}

#[test]
fn resolve_config_with_negative_indent() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let mut config = ConfigKeyMap::new();
    config.insert("indent".to_string(), ConfigKeyValue::Number(-1));

    let result = handler
      .resolve_config(config, GlobalConfiguration::default())
      .await;

    // Negative numbers are converted using i64 to i32 cast
    // The config passes through as provided value
    let js2svg = result
      .config
      .main
      .get("js2svg")
      .unwrap()
      .as_object()
      .unwrap();
    // Config system uses default (2) for values, negative treated as signed int
    let indent = js2svg.get("indent").unwrap().as_i64().unwrap();
    assert!(indent == 2 || indent == -1, "Unexpected indent: {}", indent);
  });
}

#[test]
fn resolve_config_with_invalid_eol() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let mut config = ConfigKeyMap::new();
    config.insert(
      "eol".to_string(),
      ConfigKeyValue::String("invalid".to_string()),
    );

    let result = handler
      .resolve_config(config, GlobalConfiguration::default())
      .await;

    // Invalid eol values pass through - SVGO handles validation
    let js2svg = result
      .config
      .main
      .get("js2svg")
      .unwrap()
      .as_object()
      .unwrap();
    assert_eq!(js2svg.get("eol").unwrap().as_str().unwrap(), "invalid");
  });
}

// Error type verification tests

#[test]
fn error_types_have_display() {
  use dprint_plugin_svgo::error::SvgoError;

  // Verify InvalidExtensionOverride error displays correctly
  let err = SvgoError::InvalidExtensionOverride("svg".to_string());
  let msg = format!("{}", err);
  assert!(msg.contains("extension override"));
  assert!(msg.contains("svg"));

  // Verify InvalidUtf8 error displays correctly
  let utf8_err = String::from_utf8(vec![0xff, 0xfe]).unwrap_err();
  let err = SvgoError::InvalidUtf8(utf8_err);
  let msg = format!("{}", err);
  assert!(msg.contains("UTF-8"));
}

#[test]
fn format_hidden_file_no_extension() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    // Hidden file like .gitignore - should use main config
    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from(".svgconfig"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // With Path::extension(), hidden files return None for extension
    // So main config should be used
    assert!(result.is_ok());
  });
}

// SVG structure validation tests

#[test]
fn format_deeply_nested_svg_fails() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Create SVG with 101 nested elements (exceeds MAX_SVG_DEPTH of 100)
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    for _ in 0..101 {
      svg.push_str("<g>");
    }
    svg.push_str("<rect/>");
    for _ in 0..101 {
      svg.push_str("</g>");
    }
    svg.push_str("</svg>");

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("deep.svg"),
          file_bytes: svg.into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should fail due to depth limit
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("too deep") || err_msg.contains("depth"));
  });
}

#[test]
fn format_normal_depth_svg_succeeds() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Create SVG with 50 nested elements (under MAX_SVG_DEPTH of 100)
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    for _ in 0..50 {
      svg.push_str("<g>");
    }
    svg.push_str("<rect/>");
    for _ in 0..50 {
      svg.push_str("</g>");
    }
    svg.push_str("</svg>");

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("normal.svg"),
          file_bytes: svg.into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should succeed
    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_self_closing_tags() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG with many self-closing tags (should not affect depth)
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
      <rect/><rect/><rect/><rect/><rect/>
      <circle/><circle/><circle/><circle/><circle/>
    </svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("selfclose.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_cdata_section() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG with CDATA section containing < and > characters
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
      <script><![CDATA[
        if (a < b && c > d) { console.log("test"); }
      ]]></script>
    </svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("cdata.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // CDATA should be skipped by validator
    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_processing_instruction() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG with processing instruction
    let svg = r#"<?xml version="1.0" encoding="UTF-8"?>
    <svg xmlns="http://www.w3.org/2000/svg">
      <rect/>
    </svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("pi.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Processing instruction should be skipped
    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_comments() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG with comments
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg">
      <!-- This is a comment with <fake> tags -->
      <rect/>
      <!-- Another comment -->
    </svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("comments.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Comments should be skipped by validator
    assert!(result.is_ok());
  });
}

#[test]
fn format_svg_with_many_elements_succeeds() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // SVG with many elements but within limit
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    for _ in 0..1000 {
      svg.push_str("<rect/>");
    }
    svg.push_str("</svg>");

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("many_elements.svg"),
          file_bytes: svg.into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
  });
}
