use std::borrow::Cow;
use std::path::Path;
use std::sync::OnceLock;

use deno_core::serde_json;
use dprint_core::async_runtime::async_trait;
use dprint_core::plugins::FormatRequest;
use dprint_plugin_deno_base::channel::Formatter;
use dprint_plugin_deno_base::runtime::CreateRuntimeOptions;
use dprint_plugin_deno_base::runtime::JsRuntime;
use dprint_plugin_deno_base::snapshot::deserialize_snapshot;
use dprint_plugin_deno_base::util::set_v8_max_memory;

use crate::config::SvgoConfig;
use crate::error::SvgoError;

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
    let request_value = serde_json::Value::Object({
      let mut obj = serde_json::Map::new();
      obj.insert(
        "filePath".to_string(),
        request.file_path.to_string_lossy().into(),
      );
      let file_text = String::from_utf8(request.file_bytes).map_err(SvgoError::InvalidUtf8)?;
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
    self
      .runtime
      .execute_format_script(code)
      .await
      .map(|s| s.map(std::string::String::into_bytes))
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
