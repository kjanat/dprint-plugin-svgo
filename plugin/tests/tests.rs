use std::path::PathBuf;
use std::sync::Arc;

use deno_core::futures::FutureExt;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::FormatConfigId;
use dprint_core::plugins::FormatRequest;
use dprint_core::plugins::NullCancellationToken;
use dprint_plugin_deno_base::util::create_tokio_runtime;
use dprint_plugin_svgo::SvgoPluginHandler;

// Spec tests removed - SVGO only supports SVG files
// Add SVG-specific spec tests here if needed

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
          file_bytes: "not valid svg".to_string().into_bytes(),
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
