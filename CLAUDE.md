# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Install JS dependencies (required before Rust build)
deno task setup
# or: cd js/node && bun install

# Build debug
cargo build

# Build release
cargo build --release --target <target>

# Run tests
cargo test --all-features

# Lint
cargo clippy

# Format check
cargo fmt --check
```

## Architecture

This is a dprint plugin wrapping SVGO for SVG optimization. It uses a Rust-to-JavaScript bridge via V8 (deno_core).

### Workspace Structure

**Two crates:**

- `base/` - Generic dprint plugin helpers for deno_core (runtime wrapper, thread pool, snapshot utilities)
- `plugin/` - SVGO-specific plugin implementation

### Core Flow

```
dprint CLI -> SvgoPluginHandler -> Channel (thread pool) -> JsRuntime (V8) -> SVGO optimize()
```

### Key Components

**Handler** (`plugin/src/handler.rs`): Implements `AsyncPluginHandler` trait, routes format requests to channel

**Channel** (`base/src/channel.rs`): Memory-aware thread pool that dynamically scales V8 runtimes (2.2x safety factor, 30s idle shutdown)

**Formatter** (`plugin/src/formatter.rs`): Constructs JS code with config, executes via V8, returns formatted SVG

**Config** (`plugin/src/config.rs`): Maps dprint config to SVGO js2svg options (indent, eol, pretty, multipass)

**JS Bridge** (`js/node/main.ts`): Exposes `formatText()` and `getExtensions()` to Rust via globalThis.dprint

### Build Process

`plugin/build.rs` runs `npm run build:script` to bundle JS, creates V8 snapshot, extracts supported extensions (["svg"])

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

- `scripts/create_plugin_file.ts` - Generate release plugin.json
- `scripts/output_svgo_version.ts` - Get SVGO version for release notes

## CI/Release

Multi-platform builds: macOS (x86_64, aarch64), Windows (x86_64), Linux (x86_64, aarch64)

Tag triggers release workflow that builds, creates checksums, generates plugin.json with download URLs.
