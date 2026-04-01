use std::io::Write;

use dprint_plugin_svgo::schema::SvgoConfigSchema;

fn main() {
  let mut schema = schemars::schema_for!(SvgoConfigSchema);

  schema.insert(
    "$id".to_string(),
    serde_json::json!(format!(
      "https://plugins.dprint.dev/kjanat/dprint-plugin-svgo/{}/schema.json",
      env!("CARGO_PKG_VERSION")
    )),
  );

  let json = serde_json::to_string_pretty(&schema).expect("schema serialization failed");
  let output = format!("{json}\n");

  match std::env::args().nth(1) {
    Some(path) => {
      std::fs::write(&path, &output).unwrap_or_else(|e| panic!("failed to write {path}: {e}"));
    }
    None => {
      std::io::stdout()
        .write_all(output.as_bytes())
        .expect("failed to write to stdout");
    }
  }
}
