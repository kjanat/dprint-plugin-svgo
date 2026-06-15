# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
git submodule update --init --recursive

just build               # Bundle SVGO wrapper for V8
just test                # Build + run all tests
just check               # Type-check TS + cargo clippy
just fmt                 # Format everything (dprint)
just schema              # Generate site/schema.json
just ci                  # Regenerate CI workflow YAML
just local-test          # Build release + format a disposable workspace with dprint
just update              # Check for SVGO updates + release
just verify              # Run fmt/check/test + site verification

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

**Config** (`plugin/src/config.rs`): Maps dprint config to SVGO js2svg options (indent, eol, pretty)

**JS Bridge** (`js/svgo.ts`): Exposes `formatText()` and `getExtensions()` to Rust via globalThis.dprint

**Console Shim** (`js/console.js`): Wires console methods to stderr (stdout reserved for dprint IPC)

### Build Process

`just build` bundles `js/svgo.ts` against the vendored `vendor/svgo` sources pinned by the submodule. `plugin/build.rs` creates a V8 snapshot from the final bundle and extracts supported extensions (["svg"]).

The schema is generated from `scripts/schema_types.ts` and written to `site/schema.json` for site builds. Release CI generates root `schema.json` as an artifact.

Initialize the vendored SVGO submodule after cloning with `git submodule update --init --recursive`.

No node_modules for the plugin/runtime side — Deno resolves SVGO's runtime npm dependencies via `deno.jsonc`. The `site/` directory uses Bun for install, typecheck, and build.

## Configuration

Plugin uses `"svgo"` config key in dprint.jsonc:

```jsonc
{
  "svgo": {
    "pretty": true,
    "indent": 2,
    "eol": "lf"
  }
}
```

Extension overrides: `"svg.pretty": false`

Global config integration: `indentWidth` -> indent, `newLineKind` -> eol

## Memory Management

- Default memory estimate: 100MB per isolate
- V8 max memory: 512MB
- New runtimes created only if system has 2.2x estimated memory available
- Idle runtimes shutdown after 30 seconds (except last one)

## Scripts

- `scripts/lib.ts` - Shared automation helpers used by repo scripts
- `scripts/schema_types.ts` - JSON-safe SVGO type shim used for schema generation
- `scripts/generate_schema.ts` - Generate schema from the vendored SVGO type surface and attach stable site metadata
- `scripts/create_plugin_file.ts` - Generate release plugin.json
- `scripts/output_svgo_version.ts` - Get SVGO version for release notes
- `scripts/update.ts` - Advance the SVGO submodule tag, sync Deno imports, bump version, tag release

## CI/Release

Multi-platform builds: macOS (x86_64, aarch64), Windows (x86_64), Linux (x86_64, aarch64)

Normal CI verifies the real site build on Linux. Pages builds the site explicitly with Bun and ships `dist/`.

Tag triggers release workflow that builds, creates checksums, and generates plugin.json with download URLs.
