use std::borrow::Cow;
use std::io::Read;
use std::io::Write;
use std::path::MAIN_SEPARATOR;
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
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use tokio::time::timeout;

use crate::config::SvgoConfig;
use crate::debug_log;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SvgFileKind {
  Svg,
  Svgz,
}

impl Default for SvgoFormatter {
  fn default() -> Self {
    debug_log("formatter: creating JsRuntime");
    let runtime = JsRuntime::new(CreateRuntimeOptions {
      extensions: vec![],
      startup_snapshot: Some(get_startup_snapshot()),
    });
    debug_log("formatter: JsRuntime ready");
    Self { runtime }
  }
}

#[async_trait(?Send)]
impl Formatter<SvgoConfig> for SvgoFormatter {
  async fn format_text(
    &mut self,
    request: FormatRequest<SvgoConfig>,
  ) -> Result<Option<Vec<u8>>, deno_core::anyhow::Error> {
    debug_log("formatter: format_text start");
    // Cancellation support not yet implemented. See: https://github.com/kjanat/dprint-plugin-svgo/issues/2
    // Range formatting not supported by SVGO - always formats entire document.
    let file_kind = svg_file_kind(&request.file_path);
    let file_text = decode_svg_bytes(request.file_bytes, file_kind)?;

    // Validate SVG structure before processing
    validate_svg_structure(&file_text)?;

    // Normalize file path for consistent JSON serialization across platforms
    let file_path = normalize_path_for_json(&request.file_path.to_string_lossy());
    let request_value = serde_json::Value::Object({
      let mut obj = serde_json::Map::new();
      obj.insert("filePath".to_string(), file_path.clone().into());
      obj.insert("fileText".to_string(), file_text.into());
      obj
    });
    let config = &request.config;
    let resolved_config = resolve_config(&file_path, config)?;
    let config_json =
      serde_json::to_string(&resolved_config).map_err(SvgoError::JsonSerialization)?;
    let code = format!(
      "(async () => {{ return await dprint.formatText({{ ...{}, config: {} }}); }})()",
      request_value, config_json,
    );
    let result = timeout(
      Duration::from_secs(FORMAT_TIMEOUT_SECS),
      self.runtime.execute_format_script(code),
    )
    .await
    .map_err(|_| SvgoError::Timeout {
      seconds: FORMAT_TIMEOUT_SECS,
    })?;

    let result = result?;
    let result = match result {
      Some(text) => Some(encode_svg_bytes(text, file_kind)?),
      None => None,
    };
    debug_log("formatter: format_text done");
    Ok(result)
  }
}

fn svg_file_kind(path: &Path) -> SvgFileKind {
  match path.extension().and_then(|extension| extension.to_str()) {
    Some(extension) if extension.eq_ignore_ascii_case("svgz") => SvgFileKind::Svgz,
    _ => SvgFileKind::Svg,
  }
}

fn decode_svg_bytes(bytes: Vec<u8>, file_kind: SvgFileKind) -> Result<String, SvgoError> {
  match file_kind {
    SvgFileKind::Svg => String::from_utf8(bytes).map_err(SvgoError::InvalidUtf8),
    SvgFileKind::Svgz => {
      let mut decoder = GzDecoder::new(bytes.as_slice());
      let mut decoded = Vec::new();
      decoder
        .read_to_end(&mut decoded)
        .map_err(SvgoError::SvgzDecompression)?;
      String::from_utf8(decoded).map_err(SvgoError::InvalidUtf8)
    }
  }
}

fn encode_svg_bytes(text: String, file_kind: SvgFileKind) -> Result<Vec<u8>, SvgoError> {
  match file_kind {
    SvgFileKind::Svg => Ok(text.into_bytes()),
    SvgFileKind::Svgz => {
      let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
      encoder
        .write_all(text.as_bytes())
        .map_err(SvgoError::SvgzCompression)?;
      encoder.finish().map_err(SvgoError::SvgzCompression)
    }
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

/// Normalizes a file path for consistent JSON serialization across platforms.
///
/// Converts backslashes to forward slashes for cross-platform consistency.
fn normalize_path_for_json(path: &str) -> String {
  if MAIN_SEPARATOR == '\\' {
    path.replace('\\', "/")
  } else {
    path.to_string()
  }
}
