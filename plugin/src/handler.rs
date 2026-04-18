use std::sync::Arc;
use std::sync::OnceLock;

use dprint_core::async_runtime::LocalBoxFuture;
use dprint_core::async_runtime::async_trait;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::plugins::AsyncPluginHandler;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatRequest;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::HostFormatRequest;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_plugin_deno_base::channel::Channel;
use dprint_plugin_deno_base::channel::CreateChannelOptions;

use crate::config::SvgoConfig;
use crate::config::resolve_config;
use crate::debug_log;
use crate::formatter::SvgoFormatter;

fn get_supported_extensions() -> &'static Vec<String> {
  static SUPPORTED_EXTENSIONS: OnceLock<Vec<String>> = OnceLock::new();
  SUPPORTED_EXTENSIONS.get_or_init(|| {
    let json_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/SUPPORTED_EXTENSIONS.json"));

    deno_core::serde_json::from_slice(json_bytes)
      .expect("Failed to parse SUPPORTED_EXTENSIONS.json - this is a build error")
  })
}

/// Handler for the SVGO dprint plugin.
///
/// This handler manages the formatting of SVG files using SVGO.
#[derive(Debug)]
pub struct SvgoPluginHandler {
  channel: Arc<Channel<SvgoConfig>>,
}

impl Default for SvgoPluginHandler {
  fn default() -> Self {
    Self {
      channel: Arc::new(Channel::new(CreateChannelOptions {
        avg_isolate_memory_usage: 100_000, // 100MB estimate for SVGO
        create_formatter_cb: Arc::new(|| Box::<SvgoFormatter>::default()),
      })),
    }
  }
}

#[async_trait(?Send)]
impl AsyncPluginHandler for SvgoPluginHandler {
  type Configuration = SvgoConfig;

  fn plugin_info(&self) -> PluginInfo {
    debug_log("handler: plugin_info");
    PluginInfo {
      name: env!("CARGO_PKG_NAME").to_string(),
      version: env!("CARGO_PKG_VERSION").to_string(),
      config_key: "svgo".to_string(),
      help_url: "https://svgo.dev".to_string(),
      config_schema_url: format!(
        "https://plugins.dprint.dev/kjanat/dprint-plugin-svgo/{}/schema.json",
        env!("CARGO_PKG_VERSION")
      ),
      update_url: None,
    }
  }

  fn license_text(&self) -> String {
    debug_log("handler: license_text");
    include_str!("../../LICENSE").to_string()
  }

  async fn resolve_config(
    &self,
    config: ConfigKeyMap,
    global_config: GlobalConfiguration,
  ) -> PluginResolveConfigurationResult<Self::Configuration> {
    debug_log("handler: resolve_config start");
    let result = resolve_config(config, global_config);
    let resolved = PluginResolveConfigurationResult {
      config: result.config,
      diagnostics: result.diagnostics,
      file_matching: FileMatchingInfo {
        file_extensions: get_supported_extensions().clone(),
        file_names: vec![],
      },
    };
    debug_log("handler: resolve_config done");
    resolved
  }

  async fn format(
    &self,
    request: FormatRequest<Self::Configuration>,
    _format_with_host: impl FnMut(HostFormatRequest) -> LocalBoxFuture<'static, FormatResult> + 'static,
  ) -> FormatResult {
    debug_log("handler: format start");
    if request.range.is_some() {
      // no support for range formatting
      debug_log("handler: format done");
      return Ok(None);
    }

    let result = self.channel.format(request).await;
    debug_log("handler: format done");
    result
  }
}
