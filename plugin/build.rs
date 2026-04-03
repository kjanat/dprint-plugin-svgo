//! Build script for the SVGO dprint plugin.
//!
//! Performs three steps at compile time:
//! 1. Bundles JS (main.ts + SVGO) via esbuild, invoked through `deno run -A build.ts`
//! 2. Creates a V8 heap snapshot from the bundled JS for fast runtime startup
//! 3. Extracts supported file extensions (["svg"]) by calling into the JS
//!
//! Based on patterns from Deno's codebase:
//! https://github.com/denoland/deno/blob/main/cli/build.rs

use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use deno_core::Extension;
use dprint_plugin_deno_base::runtime::CreateRuntimeOptions;
use dprint_plugin_deno_base::runtime::JsRuntime;
use dprint_plugin_deno_base::util::create_tokio_runtime;

fn main() {
  let crate_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
  let root_dir = crate_dir.parent().unwrap();
  let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  let startup_snapshot_path = out_dir.join("STARTUP_SNAPSHOT.bin");
  let js_dir = root_dir.join("js");
  let supported_extensions_path = out_dir.join("SUPPORTED_EXTENSIONS.json");

  eprintln!("Running JS build...");
  let build_result = Command::new("deno")
    .args(["task", "build"])
    .current_dir(root_dir)
    .status();
  match build_result {
    Ok(status) => {
      assert!(status.code() == Some(0), "Error building.");
    }
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
      eprintln!("Skipping build because deno executable not found.");
    }
    Err(err) => panic!("Error building to script: {err}"),
  }

  // ensure the build is invalidated if any of these files change
  println!(
    "cargo:rerun-if-changed={}",
    js_dir.join("svgo.ts").display()
  );
  println!(
    "cargo:rerun-if-changed={}",
    root_dir.join("deno.jsonc").display()
  );
  println!(
    "cargo:rerun-if-changed={}",
    root_dir.join("deno.lock").display()
  );

  let startup_code_path = js_dir.join("dist/svgo.js");
  assert!(startup_code_path.exists(), "Run `deno task build` first.");
  let snapshot = create_snapshot(startup_snapshot_path, &startup_code_path);
  let snapshot = Box::leak(snapshot);

  // serialize the supported extensions
  eprintln!("Creating runtime...");
  let tokio_runtime = create_tokio_runtime();
  JsRuntime::initialize_main_thread();
  let mut runtime = JsRuntime::new(CreateRuntimeOptions {
    extensions: vec![main::init()],
    startup_snapshot: Some(snapshot),
  });

  eprintln!("Getting extensions...");
  let file_extensions: Vec<String> = tokio_runtime.block_on(async move {
    let startup_text = get_startup_text(&startup_code_path);
    runtime
      .execute_script("dprint:svgo.js", startup_text.clone())
      .unwrap();
    runtime
      .execute_async_fn::<Vec<String>>("deno:get_extensions.js", "dprint.getExtensions".to_string())
      .await
      .unwrap()
  });
  std::fs::write(
    supported_extensions_path,
    deno_core::serde_json::to_string(&file_extensions).unwrap(),
  )
  .unwrap();
  eprintln!("Done");
}

/// Create a V8 snapshot with the bundled SVGO JS pre-loaded and executed.
/// The snapshot is compressed with zstd in release builds.
fn create_snapshot(snapshot_path: PathBuf, startup_code_path: &Path) -> Box<[u8]> {
  let startup_text = get_startup_text(startup_code_path);
  dprint_plugin_deno_base::build::create_snapshot(
    dprint_plugin_deno_base::build::CreateSnapshotOptions {
      cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
      snapshot_path,
      extensions: extensions(),
      with_runtime_cb: Some(Box::new(move |runtime| {
        runtime
          .execute_script("dprint:svgo.js", startup_text.clone())
          .unwrap();
      })),
      warmup_script: None,
    },
  )
}

/// Read the bundled JS source from `dist/main.js`.
fn get_startup_text(startup_code_path: &Path) -> String {
  std::fs::read_to_string(startup_code_path).unwrap()
}

deno_core::extension!(
  main,
  esm_entry_point = "ext:main/console.js",
  esm = [
    dir "../js",
    "console.js",
  ]
);

/// Deno extensions providing console, URL, and WebIDL APIs to the V8 runtime.
fn extensions() -> Vec<Extension> {
  vec![main::init()]
}
