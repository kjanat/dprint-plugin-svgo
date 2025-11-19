use deno_core::futures::FutureExt;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::{FormatConfigId, FormatRequest, NullCancellationToken};
use dprint_plugin_deno_base::util::create_tokio_runtime;
use dprint_plugin_svgo::config::resolve_config;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = dprint_plugin_svgo::SvgoPluginHandler::default();

    let test_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="40" fill="red" />
  <rect x="10" y="10" width="30" height="30" fill="blue" />
</svg>"#;

    // Test 1: Default config (minified)
    println!("=== Test 1: Default Config (Minified) ===");
    println!("Input:\n{}\n", test_svg);

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: test_svg.as_bytes().to_vec(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await
      .unwrap();

    if let Some(formatted) = result {
      println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
    }

    // Test 2: Pretty config (formatted with indentation)
    println!("=== Test 2: Pretty Config (Formatted) ===");
    let mut config_map = ConfigKeyMap::new();
    config_map.insert("pretty".to_string(), "true".into());
    config_map.insert("indent".to_string(), 2.into());

    let config_result = resolve_config(config_map, Default::default());

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: test_svg.as_bytes().to_vec(),
          config: Arc::new(config_result.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await
      .unwrap();

    if let Some(formatted) = result {
      println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
    }

    // Test 3: Multipass optimization
    println!("=== Test 3: Multipass Optimization ===");
    let complex_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">
  <g transform="translate(0,0)">
    <g transform="translate(10,10)">
      <circle cx="50" cy="50" r="40" fill="red" stroke="red" stroke-width="0" />
    </g>
  </g>
</svg>"#;

    println!("Input:\n{}\n", complex_svg);

    let mut config_map = ConfigKeyMap::new();
    config_map.insert("multipass".to_string(), "true".into());
    config_map.insert("pretty".to_string(), "true".into());

    let config_result = resolve_config(config_map, Default::default());

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: complex_svg.as_bytes().to_vec(),
          config: Arc::new(config_result.config),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await
      .unwrap();

    if let Some(formatted) = result {
      println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
    }

    println!("All configuration tests completed successfully!");
  });
}
