# API Reference - dprint-plugin-svgo

## Rust API

### Plugin Handler (`plugin/src/handler.rs`)

#### `SvgoPluginHandler`

Main entry point implementing dprint's `AsyncPluginHandler` trait.

```rust
pub struct SvgoPluginHandler {
  channel: Arc<Channel<SvgoConfig>>,
}
```

**Methods:**

| Method             | Description                                 | Returns                            |
| ------------------ | ------------------------------------------- | ---------------------------------- |
| `default()`        | Create handler with 100MB memory estimate   | `Self`                             |
| `plugin_info()`    | Plugin metadata (name, version, config_key) | `PluginInfo`                       |
| `license_text()`   | MIT license text                            | `String`                           |
| `resolve_config()` | Parse dprint config to SvgoConfig           | `PluginResolveConfigurationResult` |
| `format()`         | Format SVG file                             | `FormatResult`                     |

**Location**: `handler.rs:34-96`

---

### Configuration (`plugin/src/config.rs`)

#### `SvgoConfig`

Resolved configuration passed to formatter.

```rust
#[derive(Clone, Serialize, Default)]
pub struct SvgoConfig {
  pub main: serde_json::Map<String, serde_json::Value>,
  pub extension_overrides: serde_json::Map<String, serde_json::Value>,
  pub plugins: SvgoPluginConfig,
}
```

**Location**: `config.rs:17-25`

---

#### `resolve_config()`

Parse dprint ConfigKeyMap to SvgoConfig.

```rust
#[must_use]
pub fn resolve_config(
  config: ConfigKeyMap,
  global_config: GlobalConfiguration,
) -> ResolveConfigurationResult<SvgoConfig>
```

**Config Mappings:**

| dprint Key    | SVGO Key        | Default     |
| ------------- | --------------- | ----------- |
| `indentWidth` | `js2svg.indent` | 2           |
| `newLineKind` | `js2svg.eol`    | "lf"        |
| `pretty`      | `js2svg.pretty` | true        |
| `multipass`   | `multipass`     | false       |
| `indent`      | `js2svg.indent` | indentWidth |
| `eol`         | `js2svg.eol`    | newLineKind |

**Location**: `config.rs:37-132`

---

### Formatter (`plugin/src/formatter.rs`)

#### `SvgoFormatter`

Executes SVGO via V8 runtime.

```rust
pub struct SvgoFormatter {
  runtime: JsRuntime,
}
```

**Implements**: `Formatter<SvgoConfig>` trait

**Key Method:**

```rust
async fn format_text(
  &mut self,
  request: FormatRequest<SvgoConfig>,
) -> Result<Option<Vec<u8>>, Error>
```

**Location**: `formatter.rs:38-89`

---

#### `resolve_config()` (formatter)

Merge extension-specific overrides with main config.

```rust
fn resolve_config<'a>(
  file_path: &str,
  config: &'a SvgoConfig,
) -> Cow<'a, serde_json::Map<String, serde_json::Value>>
```

**Location**: `formatter.rs:91-110`

---

## Base Library API (`base/`)

### Runtime (`base/src/runtime.rs`)

#### `JsRuntime`

V8 runtime wrapper.

```rust
pub struct JsRuntime {
  inner: deno_core::JsRuntime,
}
```

**Methods:**

| Method                            | Description                             | Returns                  |
| --------------------------------- | --------------------------------------- | ------------------------ |
| `new(options)`                    | Create runtime with extensions/snapshot | `Self`                   |
| `initialize_main_thread()`        | Init V8 platform (call once)            | `()`                     |
| `execute_format_script(code)`     | Run formatting code                     | `Result<Option<String>>` |
| `execute_script(name, code)`      | Run arbitrary script                    | `Result<()>`             |
| `execute_async_fn(name, fn_name)` | Call async JS function                  | `Result<T>`              |

**Location**: `runtime.rs:26-111`

---

#### `CreateRuntimeOptions`

Options for JsRuntime creation.

```rust
pub struct CreateRuntimeOptions {
  pub extensions: Vec<Extension>,
  pub startup_snapshot: Option<&'static [u8]>,
}
```

**Location**: `runtime.rs:21-24`

---

### Channel (`base/src/channel.rs`)

#### `Channel<TConfiguration>`

Thread pool with memory-aware scaling.

```rust
pub struct Channel<TConfiguration: Send + Sync + 'static> {
  stats: Arc<Mutex<Stats>>,
  sender: async_channel::Sender<Request<TConfiguration>>,
  receiver: async_channel::Receiver<Request<TConfiguration>>,
  options: CreateChannelOptions<TConfiguration>,
}
```

**Methods:**

| Method            | Description                     | Returns        |
| ----------------- | ------------------------------- | -------------- |
| `new(options)`    | Create channel with options     | `Self`         |
| `format(request)` | Route format request to runtime | `FormatResult` |

**Location**: `channel.rs:43-137`

---

#### `CreateChannelOptions<TConfiguration>`

Channel configuration.

```rust
pub struct CreateChannelOptions<TConfiguration> {
  pub avg_isolate_memory_usage: usize,
  pub create_formatter_cb: Arc<CreateFormatterCb<TConfiguration>>,
}
```

**Location**: `channel.rs:25-34`

---

#### `Formatter<TConfiguration>` Trait

Interface for formatters.

```rust
#[async_trait(?Send)]
pub trait Formatter<TConfiguration> {
  async fn format_text(
    &mut self,
    request: FormatRequest<TConfiguration>,
  ) -> Result<Option<Vec<u8>>, Error>;
}
```

**Location**: `channel.rs:14-20`

---

## JavaScript API (`js/node/main.ts`)

### Global Object

```typescript
globalThis.dprint = {
  getExtensions,
  formatText,
};
```

---

### `getExtensions()`

Return supported file extensions.

```typescript
async function getExtensions(): Promise<string[]>;
// Returns: ["svg"]
```

---

### `formatText()`

Format SVG content using SVGO.

```typescript
interface FormatTextOptions {
  filePath: string;
  fileText: string;
  config: Config;
  pluginsConfig: PluginsConfig;
}

async function formatText(
  options: FormatTextOptions,
): Promise<string | undefined>;
```

**Returns:**

- Formatted string if changed
- `undefined` if unchanged or error

**Location**: `js/node/main.ts:24-44`

---

## Internal Types

### Stats (`base/src/channel.rs`)

Thread pool statistics.

```rust
struct Stats {
  pending_runtimes: usize,
  total_runtimes: usize,
}
```

**Location**: `channel.rs:38-41`

---

### Request Type

Channel request format.

```rust
type Request<TConfiguration> = (FormatRequest<TConfiguration>, oneshot::Sender<FormatResult>);
```

**Location**: `channel.rs:36`

---

## dprint-core Types Used

From `dprint_core::plugins`:

- `AsyncPluginHandler` - Main plugin trait
- `FormatRequest` - Input for format operation
- `FormatResult` - Output type (`Result<Option<Vec<u8>>>`)
- `PluginInfo` - Plugin metadata
- `FileMatchingInfo` - Supported extensions/filenames
- `PluginResolveConfigurationResult` - Config resolution result

From `dprint_core::configuration`:

- `ConfigKeyMap` - Raw config from dprint
- `GlobalConfiguration` - Global dprint settings
- `NewLineKind` - Line ending enum
- `ResolveConfigurationResult` - Config result with diagnostics

---

## Error Types

All errors use `deno_core::anyhow::Error`:

```rust
use deno_core::anyhow::{Error, Result, anyhow};
```

Common error scenarios:

- Invalid UTF-8 in file bytes
- V8 execution failure
- Serde deserialization failure

---

## Memory Constants

| Constant                   | Value           | Location          |
| -------------------------- | --------------- | ----------------- |
| `avg_isolate_memory_usage` | 100,000 (100MB) | `handler.rs:42`   |
| V8 max memory              | 512 MB          | `formatter.rs:28` |
| Memory safety factor       | 2.2x            | `channel.rs:97`   |
| Idle shutdown timeout      | 30 seconds      | `channel.rs:112`  |

---

## Build-Time APIs

### Snapshot Creation (`plugin/build.rs`)

```rust
fn create_snapshot(
  snapshot_path: PathBuf,
  startup_code_path: &Path
) -> Box<[u8]>
```

Uses `dprint_plugin_deno_base::build::create_snapshot()` with:

- Deno extensions (webidl, console, url)
- Custom main extension for polyfills

**Location**: `plugin/build.rs:96-111`

---

## Cross-References

### Configuration Flow

`config.rs:resolve_config` -> `formatter.rs:resolve_config` -> `main.ts:formatText`

### Format Request Flow

`handler.rs:format` -> `channel.rs:format` -> `formatter.rs:format_text` -> `runtime.rs:execute_format_script`

### Runtime Initialization

`handler.rs:default` -> `channel.rs:new` -> (lazy) `formatter.rs:default` -> `runtime.rs:new`

---

_API Reference for dprint-plugin-svgo v0.1.0_
