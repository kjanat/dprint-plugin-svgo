use std::path::PathBuf;

use deno_core::Extension;
use deno_core::JsRuntimeForSnapshot;

pub type WithRuntimeCb = dyn Fn(&mut JsRuntimeForSnapshot);

pub struct CreateSnapshotOptions {
  pub cargo_manifest_dir: &'static str,
  pub snapshot_path: PathBuf,
  pub extensions: Vec<Extension>,
  pub with_runtime_cb: Option<Box<WithRuntimeCb>>,
  pub warmup_script: Option<&'static str>,
}

/// Creates a snapshot, returning the uncompressed bytes.
///
/// # Panics
///
/// Panics if snapshot creation fails or the output file cannot be written.
#[must_use]
pub fn create_snapshot(options: CreateSnapshotOptions) -> Box<[u8]> {
  let snapshot_output = deno_core::snapshot::create_snapshot(
    deno_core::snapshot::CreateSnapshotOptions {
      cargo_manifest_dir: options.cargo_manifest_dir,
      startup_snapshot: None,
      extensions: options.extensions,
      skip_op_registration: false,
      with_runtime_cb: options.with_runtime_cb,
      extension_transpiler: None,
    },
    options.warmup_script,
  )
  .expect("failed to create V8 snapshot");

  let output: Box<[u8]> = if cfg!(debug_assertions) {
    // In debug mode, use uncompressed snapshot for faster builds
    snapshot_output.output.clone()
  } else {
    // In release mode, compress the snapshot
    eprintln!("Compressing snapshot...");
    let uncompressed = &snapshot_output.output;
    let mut vec = Vec::with_capacity(uncompressed.len());
    vec.extend((uncompressed.len() as u32).to_le_bytes());
    vec.extend_from_slice(
      &zstd::bulk::compress(uncompressed, 22).expect("snapshot compression failed"),
    );
    vec.into()
  };

  std::fs::write(&options.snapshot_path, &output).unwrap_or_else(|e| {
    panic!(
      "failed to write snapshot to {}: {}",
      options.snapshot_path.display(),
      e
    )
  });

  for file in snapshot_output.files_loaded_during_snapshot {
    println!("cargo:rerun-if-changed={}", file.display());
  }

  snapshot_output.output
}
