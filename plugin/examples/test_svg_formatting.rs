use deno_core::futures::FutureExt;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::{FormatConfigId, FormatRequest, NullCancellationToken};
use dprint_plugin_deno_base::util::create_tokio_runtime;
use std::path::PathBuf;
use std::sync::Arc;

fn main() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = dprint_plugin_svgo::SvgoPluginHandler::default();

    // Test 1: Simple SVG with extra whitespace
    let svg1 = r#"<svg   xmlns="http://www.w3.org/2000/svg"   viewBox="0 0 50 50"  >
  <rect   x="5"  y="5"   width="40"   height="40"  fill="red"  />
  <circle  cx="25"  cy="25"  r="15"  fill="white"  stroke="blue"  stroke-width="1"  />
</svg>"#;

    println!("Testing SVG 1 (extra whitespace):");
    println!("Input:\n{}\n", svg1);

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test1.svg"),
          file_bytes: svg1.as_bytes().to_vec(),
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
    } else {
      println!("No changes needed\n");
    }

    // Test 2: SVG with comment and metadata
    let svg2 = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <defs>
    <style>
      .cls-1 { fill: #ff0000; }
    </style>
  </defs>
  <!-- This is a comment -->
  <circle cx="50" cy="50" r="40" class="cls-1" />
</svg>"#;

    println!("Testing SVG 2 (with XML declaration and comment):");
    println!("Input:\n{}\n", svg2);

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test2.svg"),
          file_bytes: svg2.as_bytes().to_vec(),
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
    } else {
      println!("No changes needed\n");
    }

    println!("All tests completed successfully!");
  });
}
