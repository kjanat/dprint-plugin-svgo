# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
deno task build          # Bundle SVGO wrapper for V8
deno task test           # Build + run all tests
deno task check          # Type-check TS + cargo clippy
deno task fmt            # Format everything (dprint)
deno task schema         # Generate JSON Schema
deno task ci             # Regenerate CI workflow YAML
deno task local-test     # Build release + test with dprint
deno task update         # Check for SVGO updates + release

cargo build              # Build debug
cargo build --release    # Build release
```

## Architecture

This is a dprint plugin wrapping SVGO for SVG optimization. It uses a Rust-to-JavaScript bridge via V8 (deno_core).

### Workspace Structure

**Two crates:**

- `base/` - Generic dprint plugin helpers for deno_core (runtime wrapper, thread pool, snapshot utilities)
- `plugin/` - SVGO-specific plugin implementation

### Core Flow

```text
dprint CLI -> SvgoPluginHandler -> Channel (thread pool) -> JsRuntime (V8) -> SVGO optimize()
```

### Key Components

**Handler** (`plugin/src/handler.rs`): Implements `AsyncPluginHandler` trait, routes format requests to channel

**Channel** (`base/src/channel.rs`): Memory-aware thread pool that dynamically scales V8 runtimes (2.2x safety factor, 30s idle shutdown)

**Formatter** (`plugin/src/formatter.rs`): Constructs JS code with config, executes via V8, returns formatted SVG

**Config** (`plugin/src/config.rs`): Maps dprint config to SVGO js2svg options (indent, eol, pretty, multipass)

**JS Bridge** (`js/svgo.ts`): Exposes `formatText()` and `getExtensions()` to Rust via globalThis.dprint

**Console Shim** (`js/console.js`): Wires console methods to stderr (stdout reserved for dprint IPC)

### Build Process

`deno task build` bundles `js/svgo.ts` + SVGO via `deno bundle` into a single IIFE. `plugin/build.rs` creates a V8 snapshot from the bundle and extracts supported extensions (["svg"]).

No node_modules — Deno resolves npm packages on the fly.

## Configuration

Plugin uses `"svgo"` config key in dprint.jsonc:

```jsonc
{
  "svgo": {
    "multipass": true,
    "pretty": true,
    "indent": 2,
    "eol": "lf"
  }
}
```

Extension overrides: `"svg.multipass": false`

Global config integration: `indentWidth` -> indent, `newLineKind` -> eol

## Memory Management

- Default memory estimate: 100MB per isolate
- V8 max memory: 512MB
- New runtimes created only if system has 2.2x estimated memory available
- Idle runtimes shutdown after 30 seconds (except last one)

## Scripts

- `scripts/generate_schema.ts` - Generate JSON Schema from SVGO's plugin registry
- `scripts/create_plugin_file.ts` - Generate release plugin.json
- `scripts/output_svgo_version.ts` - Get SVGO version for release notes
- `scripts/update.ts` - Check for SVGO updates, bump version, tag release

## CI/Release

Multi-platform builds: macOS (x86_64, aarch64), Windows (x86_64), Linux (x86_64, aarch64)

Tag triggers release workflow that builds, creates checksums, generates plugin.json with download URLs.
