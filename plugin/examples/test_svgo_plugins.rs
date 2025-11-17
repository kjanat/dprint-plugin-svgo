use std::sync::Arc;
use std::path::PathBuf;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::{FormatRequest, FormatConfigId, NullCancellationToken};
use dprint_core::configuration::ConfigKeyMap;
use dprint_plugin_deno_base::util::create_tokio_runtime;
use deno_core::futures::FutureExt;
use dprint_plugin_svgo::config::resolve_config;

fn main() {
    let runtime = create_tokio_runtime();

    runtime.block_on(async {
        let handler = dprint_plugin_svgo::SvgoPluginHandler::default();

        let test_svg = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- This is a comment -->
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100" width="100" height="100">
  <title>Test SVG</title>
  <desc>A test SVG file</desc>
  <defs>
    <style>
      .red { fill: #FF0000; }
      .blue { fill: #0000FF; }
    </style>
  </defs>
  <rect x="10" y="10" width="30" height="30" class="red" stroke="none" fill-opacity="1.0"/>
  <circle cx="50" cy="50" r="20" class="blue" transform="translate(0, 0)"/>
  <ellipse cx="75" cy="75" rx="10" ry="10" fill="green"/>
</svg>"#;

        println!("=== Test 1: Default SVGO plugins (preset-default) ===");
        let result = handler.format(
            FormatRequest {
                config_id: FormatConfigId::from_raw(0),
                file_path: PathBuf::from("test.svg"),
                file_bytes: test_svg.as_bytes().to_vec(),
                config: Arc::new(Default::default()),
                range: None,
                token: Arc::new(NullCancellationToken),
            },
            |_| std::future::ready(Ok(None)).boxed_local(),
        ).await.unwrap();

        if let Some(formatted) = result {
            println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
        }

        println!("=== Test 2: Disable all plugins, enable only removeComments ===");
        let mut config_map = ConfigKeyMap::new();
        let plugins_config = serde_json::json!([
            {
                "name": "preset-default",
                "params": {
                    "overrides": {
                        "removeComments": true,
                        "removeTitle": false,
                        "removeDesc": false,
                        "removeDoctype": false,
                        "removeXMLProcInst": false,
                        "minifyStyles": false,
                        "convertColors": false
                    }
                }
            }
        ]);
        config_map.insert("plugins".to_string(), serde_json::to_string(&plugins_config).unwrap().into());
        config_map.insert("pretty".to_string(), "true".into());

        let config_result = resolve_config(config_map, Default::default());

        let result = handler.format(
            FormatRequest {
                config_id: FormatConfigId::from_raw(0),
                file_path: PathBuf::from("test.svg"),
                file_bytes: test_svg.as_bytes().to_vec(),
                config: Arc::new(config_result.config),
                range: None,
                token: Arc::new(NullCancellationToken),
            },
            |_| std::future::ready(Ok(None)).boxed_local(),
        ).await.unwrap();

        if let Some(formatted) = result {
            println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
        }

        println!("=== Test 3: Custom plugin combination ===");
        let mut config_map = ConfigKeyMap::new();
        let plugins_config = serde_json::json!([
            {
                "name": "removeComments"
            },
            {
                "name": "removeTitle"
            },
            {
                "name": "removeDesc"
            },
            {
                "name": "removeDimensions"
            },
            {
                "name": "convertColors",
                "params": {
                    "currentColor": true
                }
            },
            {
                "name": "convertTransform"
            },
            {
                "name": "cleanupIds"
            }
        ]);
        config_map.insert("plugins".to_string(), serde_json::to_string(&plugins_config).unwrap().into());
        config_map.insert("pretty".to_string(), "true".into());
        config_map.insert("indent".to_string(), 2.into());

        let config_result = resolve_config(config_map, Default::default());

        let result = handler.format(
            FormatRequest {
                config_id: FormatConfigId::from_raw(0),
                file_path: PathBuf::from("test.svg"),
                file_bytes: test_svg.as_bytes().to_vec(),
                config: Arc::new(config_result.config),
                range: None,
                token: Arc::new(NullCancellationToken),
            },
            |_| std::future::ready(Ok(None)).boxed_local(),
        ).await.unwrap();

        if let Some(formatted) = result {
            println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
        }

        println!("=== Test 4: All optimization plugins enabled ===");
        let mut config_map = ConfigKeyMap::new();
        let plugins_config = serde_json::json!([
            "removeDoctype",
            "removeXMLProcInst",
            "removeComments",
            "removeMetadata",
            "removeEditorsNSData",
            "cleanupAttrs",
            "mergeStyles",
            "inlineStyles",
            "minifyStyles",
            "cleanupIds",
            "removeUselessDefs",
            "cleanupNumericValues",
            "convertColors",
            "removeUnknownsAndDefaults",
            "removeNonInheritableGroupAttrs",
            "removeUselessStrokeAndFill",
            "removeViewBox",
            "cleanupEnableBackground",
            "removeHiddenElems",
            "removeEmptyText",
            "convertShapeToPath",
            "convertEllipseToCircle",
            "moveElemsAttrsToGroup",
            "moveGroupAttrsToElems",
            "collapseGroups",
            "convertPathData",
            "convertTransform",
            "removeEmptyAttrs",
            "removeEmptyContainers",
            "mergePaths",
            "removeUnusedNS",
            "sortAttrs",
            "sortDefsChildren",
            "removeTitle",
            "removeDesc"
        ]);
        config_map.insert("plugins".to_string(), serde_json::to_string(&plugins_config).unwrap().into());
        config_map.insert("multipass".to_string(), "true".into());
        config_map.insert("pretty".to_string(), "true".into());

        let config_result = resolve_config(config_map, Default::default());

        let result = handler.format(
            FormatRequest {
                config_id: FormatConfigId::from_raw(0),
                file_path: PathBuf::from("test.svg"),
                file_bytes: test_svg.as_bytes().to_vec(),
                config: Arc::new(config_result.config),
                range: None,
                token: Arc::new(NullCancellationToken),
            },
            |_| std::future::ready(Ok(None)).boxed_local(),
        ).await.unwrap();

        if let Some(formatted) = result {
            println!("Output:\n{}\n", String::from_utf8(formatted).unwrap());
        }

        println!("All plugin configuration tests completed successfully!");
    });
}
