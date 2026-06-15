use dprint_core::plugins::process::get_parent_process_id_from_cli_args;
use dprint_core::plugins::process::start_parent_process_checker_task;
use dprint_plugin_deno_base::runtime::JsRuntime;
use dprint_plugin_deno_base::util::create_tokio_runtime;
use dprint_plugin_svgo::SvgoPluginHandler;
use dprint_plugin_svgo::debug_log;
use dprint_plugin_svgo::handle_process_stdio_messages_sync;

fn main() {
  let is_init_process = std::env::args().any(|arg| arg == "--init");
  if is_init_process {
    debug_log("main: skipping V8 platform init for --init process");
  } else {
    debug_log("main: initializing V8 platform");
    JsRuntime::initialize_main_thread();
  }
  debug_log("main: creating tokio runtime");
  let runtime = create_tokio_runtime();
  let result = runtime.block_on(async {
    if let Some(parent_process_id) = get_parent_process_id_from_cli_args() {
      debug_log("main: starting parent process checker");
      start_parent_process_checker_task(parent_process_id);
    }

    debug_log("main: entering process stdio message loop");
    handle_process_stdio_messages_sync(SvgoPluginHandler::default()).await
  });

  if let Err(err) = result {
    eprintln!("Shutting down due to error: {err}");
    std::process::exit(1);
  }
}
