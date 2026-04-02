# dprint-plugin-svgo - Project Documentation

## Overview

A dprint plugin that wraps SVGO (SVG Optimizer) to provide SVG optimization as part of the dprint formatting ecosystem. Enables parallel SVG formatting alongside other dprint-supported languages.

**Repository**: https://github.com/kjanat/dprint-plugin-svgo
**License**: MIT
**Rust Edition**: 2024

---

## Architecture

### High-Level Design

```
+------------------+     +------------------+     +------------------+
|    dprint CLI    | --> | Plugin Handler   | --> |   V8 Runtime     |
|                  |     | (Rust/Async)     |     |   (deno_core)    |
+------------------+     +------------------+     +------------------+
                                                          |
                                                          v
                                                  +------------------+
                                                  |  SVGO (JS/TS)    |
                                                  |  Optimizer       |
                                                  +------------------+
```

### Core Components

The project is a **Rust workspace** with two crates:

#### 1. `base` (dprint-plugin-deno-base)

Helper library for creating dprint plugins with deno_core.

**Key modules**:

- `runtime.rs` - JsRuntime wrapper for V8 execution
- `channel.rs` - Thread pool management with memory-aware scaling
- `snapshot.rs` - V8 snapshot serialization/deserialization
- `util.rs` - System utilities (memory detection, tokio runtime)
- `build.rs` - Build-time snapshot creation

#### 2. `plugin` (dprint-plugin-svgo)

The actual dprint plugin implementation.

**Key modules**:

- `handler.rs` - AsyncPluginHandler implementation
- `formatter.rs` - SVGO formatting logic via V8
- `config.rs` - Configuration resolution from dprint

---

## Data Flow

### Format Request Flow

```
1. dprint CLI calls plugin.format()
2. SvgoPluginHandler receives FormatRequest
3. Channel routes request to available JS runtime
4. SvgoFormatter constructs JS code with config
5. V8 executes SVGO optimize()
6. Result returned through oneshot channel
7. Formatted SVG or None if unchanged
```

### Configuration Flow

```
1. dprint.jsonc "svgo" section
2. resolve_config() in config.rs
3. Maps to SVGO js2svg options:
   - indent -> js2svg.indent
   - newLineKind -> js2svg.eol
   - pretty -> js2svg.pretty
   - multipass -> multipass
4. Extension overrides (svg.multipass)
```

---

## Key Files Reference

### Rust Source

| File                             | Purpose                              | Lines |
| -------------------------------- | ------------------------------------ | ----- |
| `plugin/src/handler.rs:34-96`    | AsyncPluginHandler impl, plugin info | 62    |
| `plugin/src/formatter.rs:38-110` | SvgoFormatter with V8 execution      | 72    |
| `plugin/src/config.rs:38-151`    | Configuration resolution             | 113   |
| `base/src/channel.rs:43-137`     | Thread pool with memory management   | 94    |
| `base/src/runtime.rs:26-111`     | JsRuntime wrapper                    | 85    |

### JavaScript Source

| File              | Purpose                                   |
| ----------------- | ----------------------------------------- |
| `js/node/main.ts` | SVGO bridge with formatText/getExtensions |
| `js/main.js`      | Global polyfills for deno_core            |

### Build & CI

| File                            | Purpose                                 |
| ------------------------------- | --------------------------------------- |
| `plugin/build.rs`               | Snapshot creation, extension extraction |
| `.github/workflows/ci.yml`      | Multi-platform builds (5 targets)       |
| `scripts/create_plugin_file.ts` | Release plugin.json generation          |

---

## Configuration Options

### Basic Configuration

```jsonc
{
  "svgo": {
    "multipass": true, // Multiple optimization passes
    "pretty": true, // Pretty-print output
    "indent": 2, // Indentation width
    "eol": "lf" // Line endings (lf/crlf)
  }
}
```

### Extension Overrides

```jsonc
{
  "svgo": {
    "multipass": true,
    "svg.multipass": false // Override for .svg files
  }
}
```

### Global Configuration Integration

- `indentWidth` -> `indent`
- `newLineKind` -> `eol`

---

## Thread Pool & Memory Management

### Channel Architecture (`base/src/channel.rs`)

The plugin uses a smart thread pool that:

1. **Lazy Runtime Creation**: First request creates a JS runtime
2. **Memory-Aware Scaling**: New runtimes only if 2.2x avg memory available
3. **Auto-Shutdown**: Idle runtimes shutdown after 30 seconds
4. **Request Routing**: Unbounded async channel distributes work

```rust
// Memory threshold check
available_memory > (avg_isolate_memory_usage * 2.2)

// Default SVGO memory estimate
avg_isolate_memory_usage: 100_000  // 100MB
```

### V8 Configuration

- Max memory: 512MB (`set_v8_max_memory(512)`)
- Single-threaded V8 platform
- Compressed snapshots for fast startup

---

## Build Process

### JavaScript Bundling

```bash
cd js/node && deno run -A build.ts  # Bundles main.ts via esbuild
```

Produces `js/node/dist/main.js` bundled for browser context.

### Rust Build

```bash
cargo build --release --target <target>
```

The `plugin/build.rs`:

1. Runs `deno run -A build.ts` (bundles JS via esbuild)
2. Creates V8 snapshot from bundled JS
3. Extracts supported extensions (["svg"])
4. Writes `STARTUP_SNAPSHOT.bin` and `SUPPORTED_EXTENSIONS.json`

### Supported Targets

- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`

---

## Dependencies

### Workspace Dependencies

```toml
deno_console = "0.184.0"
deno_core    = "0.326.0"
deno_url     = "0.184.0"
deno_webidl  = "0.184.0"
dprint-core  = "0.67.4"
serde        = "1.0.228"
serde_json   = "1"
zstd         = "0.13.3"
```

### JavaScript Dependencies

```json
"svgo": "^4.0.0"
"buffer": "^6.0.3"
"process": "^0.11.10"
```

---

## Testing

### Unit Tests (`plugin/tests/tests.rs`)

```bash
cargo test --all-features
```

Tests:

- `handle_invalid_svg` - Graceful handling of invalid input
- `handle_valid_svg` - Successful optimization

### Examples (`plugin/examples/`)

- `test_svgo_plugins.rs`
- `test_svg_formatting.rs`
- `test_svg_config.rs`

---

## Release Process

1. Tag triggers CI workflow
2. Build on all 5 targets
3. Create zip artifacts with checksums
4. Generate `plugin.json` with URLs
5. GitHub release with install instructions

### Plugin File Format

```json
{
  "schemaVersion": 1,
  "kind": "process",
  "name": "dprint-plugin-svgo",
  "version": "<version>",
  "configKey": "svgo",
  "fileExtensions": ["svg"],
  "archives": {
    "<target>": {
      "checksum": "<sha256>",
      "url": "<release_url>"
    }
  }
}
```

---

## Performance Considerations

### Advantages

1. **Parallel Processing**: Multiple files formatted concurrently
2. **Incremental Formatting**: dprint caches unchanged files
3. **Memory-Aware**: Prevents OOM by limiting runtime instances
4. **Fast Startup**: V8 snapshots avoid JS parsing overhead

### Optimization Tips

- Use `multipass: true` for maximum optimization
- Set `pretty: false` for production builds (smaller output)
- Memory-limited systems: `DPRINT_MAX_THREADS=1`

---

## Integration Patterns

### With dprint

```jsonc
// dprint.jsonc
{
  "includes": ["**/*.svg"],
  "plugins": [
    "https://plugins.dprint.dev/svgo-<version>.json@<checksum>"
  ],
  "svgo": {
    "multipass": true,
    "pretty": true
  }
}
```

### Command Line

```bash
dprint fmt           # Format all
dprint fmt file.svg  # Format specific
dprint check         # Check formatting
```

---

## Development Workflow

### Setup

```bash
# Install dependencies
cd js/node && deno install

# Build debug
cargo build

# Run tests
cargo test --all-features

# Lint
cargo clippy
```

### Scripts

| Script                           | Purpose                    |
| -------------------------------- | -------------------------- |
| `scripts/create_for_testing.ts`  | Create test plugin file    |
| `scripts/create_plugin_file.ts`  | Create release plugin.json |
| `scripts/local_test.ts`          | Local testing utilities    |
| `scripts/output_svgo_version.ts` | Get SVGO version           |
| `scripts/update.ts`              | Update dependencies        |

---

## Error Handling

### Rust Side

- Format errors return `Err(Error)`
- Invalid SVG returns `Ok(None)` (keeps original)
- Range formatting returns `Ok(None)` (unsupported)

### JavaScript Side

```typescript
try {
  const result = optimize(fileText, config);
  // ...
} catch (error) {
  console.error(`SVGO error for ${filePath}:`, error);
  return undefined; // Keep original file
}
```

---

## Extension Points

### Adding Configuration Options

1. Add to `SvgoConfig` struct (`config.rs`)
2. Parse in `resolve_config()` function
3. Pass to JS via `serde_json::to_string()`
4. Use in `formatText()` (`main.ts`)

### Custom SVGO Plugins

Extend `SvgoPluginConfig` and pass through `pluginsConfig`:

```rust
pub struct SvgoPluginConfig {
  // Custom fields here
}
```

---

## Known Limitations

1. **No Range Formatting**: Full file only
2. **No Cancellation**: Requests cannot be cancelled
3. **SVG Only**: Single file extension support
4. **Memory Estimation**: 100MB is approximate

---

## Troubleshooting

### Build Fails

```bash
# Ensure JS is built first
cd js/node && deno run -A build.ts
```

### OOM Errors

```bash
# Limit concurrent runtimes
DPRINT_MAX_THREADS=1 dprint fmt
```

### Invalid Output

Check SVGO configuration - some plugins may produce invalid SVG for certain inputs.

---

## Related Projects

- [dprint](https://dprint.dev/) - Code formatter
- [SVGO](https://svgo.dev/) - SVG optimizer
- [deno_core](https://github.com/denoland/deno_core) - JS runtime

---

_Generated for dprint-plugin-svgo v0.1.0_
