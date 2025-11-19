use std::borrow::Cow;
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use deno_core::serde_json;
use dprint_core::async_runtime::async_trait;
use dprint_core::plugins::FormatRequest;
use dprint_plugin_deno_base::channel::Formatter;
use dprint_plugin_deno_base::runtime::CreateRuntimeOptions;
use dprint_plugin_deno_base::runtime::JsRuntime;
use dprint_plugin_deno_base::snapshot::deserialize_snapshot;
use dprint_plugin_deno_base::util::set_v8_max_memory;
use tokio::time::timeout;

use crate::config::SvgoConfig;
use crate::error::SvgoError;

/// Maximum allowed nesting depth in SVG structure.
const MAX_SVG_DEPTH: usize = 100;

/// Maximum allowed number of elements in SVG.
const MAX_SVG_ELEMENTS: usize = 100_000;

/// Timeout for format operations in seconds.
const FORMAT_TIMEOUT_SECS: u64 = 30;

fn get_startup_snapshot() -> &'static [u8] {
  // Copied from Deno's codebase:
  // https://github.com/denoland/deno/blob/daa7c6d32ab5a4029f8084e174d621f5562256be/cli/tsc.rs#L55
  static STARTUP_SNAPSHOT: OnceLock<Box<[u8]>> = OnceLock::new();

  STARTUP_SNAPSHOT.get_or_init(
    #[cold]
    #[inline(never)]
    || {
      // Also set the v8 max memory at the same time. This was added because
      // on the DefinitelyTyped repo there would be some OOM errors after formatting
      // for a while and this solved that for some reason.
      set_v8_max_memory(512);

      static COMPRESSED_COMPILER_SNAPSHOT: &[u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/STARTUP_SNAPSHOT.bin"));

      deserialize_snapshot(COMPRESSED_COMPILER_SNAPSHOT)
        .expect("Failed to deserialize V8 snapshot - this is a build error")
    },
  )
}

/// SVGO formatter that wraps a V8 JavaScript runtime for SVG optimization.
///
/// This struct manages a Deno-based V8 runtime that executes SVGO to optimize SVG files.
/// Each instance maintains its own isolated runtime for thread safety.
pub struct SvgoFormatter {
  runtime: JsRuntime,
}

impl Default for SvgoFormatter {
  fn default() -> Self {
    let runtime = JsRuntime::new(CreateRuntimeOptions {
      extensions: vec![
        deno_webidl::deno_webidl::init_ops(),
        deno_console::deno_console::init_ops(),
        deno_url::deno_url::init_ops(),
      ],
      startup_snapshot: Some(get_startup_snapshot()),
    });
    Self { runtime }
  }
}

#[async_trait(?Send)]
impl Formatter<SvgoConfig> for SvgoFormatter {
  async fn format_text(
    &mut self,
    request: FormatRequest<SvgoConfig>,
  ) -> Result<Option<Vec<u8>>, deno_core::anyhow::Error> {
    // TODO(#future): Cancellation support requires passing token to V8 runtime.
    // Range formatting not supported by SVGO - always formats entire document.
    let file_text = String::from_utf8(request.file_bytes).map_err(SvgoError::InvalidUtf8)?;

    // Validate SVG structure before processing
    validate_svg_structure(&file_text)?;

    let request_value = serde_json::Value::Object({
      let mut obj = serde_json::Map::new();
      obj.insert(
        "filePath".to_string(),
        request.file_path.to_string_lossy().into(),
      );
      obj.insert("fileText".to_string(), file_text.into());
      obj
    });
    let file_path = request.file_path.to_string_lossy();
    let config = &request.config;
    let resolved_config = resolve_config(&file_path, config)?;
    let config_json =
      serde_json::to_string(&resolved_config).map_err(SvgoError::JsonSerialization)?;
    let plugins_json =
      serde_json::to_string(&config.plugins).map_err(SvgoError::JsonSerialization)?;
    let code = format!(
      "(async () => {{ return await dprint.formatText({{ ...{}, config: {}, pluginsConfig: {} }}); }})()",
      request_value, config_json, plugins_json,
    );
    let result = timeout(
      Duration::from_secs(FORMAT_TIMEOUT_SECS),
      self.runtime.execute_format_script(code),
    )
    .await
    .map_err(|_| SvgoError::Timeout {
      seconds: FORMAT_TIMEOUT_SECS,
    })?;

    result.map(|s| s.map(std::string::String::into_bytes))
  }
}

fn resolve_config<'a>(
  file_path: &str,
  config: &'a SvgoConfig,
) -> Result<Cow<'a, serde_json::Map<String, serde_json::Value>>, SvgoError> {
  let ext = match Path::new(file_path).extension().and_then(|e| e.to_str()) {
    Some(e) => e.to_lowercase(),
    None => return Ok(Cow::Borrowed(&config.main)),
  };

  match config.extension_overrides.get(&ext) {
    None => Ok(Cow::Borrowed(&config.main)),
    Some(override_config) => {
      let override_obj = override_config
        .as_object()
        .ok_or_else(|| SvgoError::InvalidExtensionOverride(ext.clone()))?;
      let mut new_config = config.main.clone();
      for (key, value) in override_obj {
        new_config.insert(key.to_string(), value.clone());
      }
      Ok(Cow::Owned(new_config))
    }
  }
}

/// Validates SVG structure to prevent malicious deeply nested content.
///
/// Checks for:
/// - Maximum nesting depth (prevents stack exhaustion)
/// - Maximum element count (prevents memory exhaustion)
fn validate_svg_structure(content: &str) -> Result<(), SvgoError> {
  let mut depth: usize = 0;
  let mut max_depth: usize = 0;
  let mut element_count: usize = 0;
  let mut in_tag = false;
  let mut is_closing = false;
  let mut is_self_closing = false;
  let bytes = content.as_bytes();
  let len = bytes.len();
  let mut i = 0;

  while i < len {
    let c = bytes[i];

    match c {
      b'<' => {
        in_tag = true;
        is_closing = false;
        is_self_closing = false;

        // Check if it's a closing tag
        if i + 1 < len && bytes[i + 1] == b'/' {
          is_closing = true;
        }
        // Skip comments and CDATA
        else if i + 3 < len && bytes[i + 1] == b'!' {
          // Skip to end of comment/CDATA
          while i < len && !(bytes[i] == b'>' && i > 0 && bytes[i - 1] == b'-') {
            i += 1;
          }
          in_tag = false;
        }
        // Skip processing instructions
        else if i + 1 < len && bytes[i + 1] == b'?' {
          while i < len && bytes[i] != b'>' {
            i += 1;
          }
          in_tag = false;
        }
      }
      b'/' if in_tag => {
        // Check for self-closing tag
        if i + 1 < len && bytes[i + 1] == b'>' {
          is_self_closing = true;
        }
      }
      b'>' if in_tag => {
        in_tag = false;

        if is_closing {
          // Closing tag - decrease depth
          depth = depth.saturating_sub(1);
        } else if is_self_closing {
          // Self-closing tag - count but don't change depth
          element_count += 1;
          if element_count > MAX_SVG_ELEMENTS {
            return Err(SvgoError::MaxElementsExceeded {
              max: MAX_SVG_ELEMENTS,
            });
          }
        } else {
          // Opening tag - increase depth
          element_count += 1;
          if element_count > MAX_SVG_ELEMENTS {
            return Err(SvgoError::MaxElementsExceeded {
              max: MAX_SVG_ELEMENTS,
            });
          }
          depth += 1;
          if depth > max_depth {
            max_depth = depth;
          }
          if max_depth > MAX_SVG_DEPTH {
            return Err(SvgoError::MaxDepthExceeded { max: MAX_SVG_DEPTH });
          }
        }
      }
      _ => {}
    }

    i += 1;
  }

  Ok(())
}
