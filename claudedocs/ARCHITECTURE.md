# Architecture Overview

## Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        dprint CLI                           │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────┐
│                    SvgoPluginHandler                       │
│  ┌────────────────────────────────────────────────────┐    │
│  │                    Channel                         │    │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │    │
│  │  │Runtime 1│  │Runtime 2│  │Runtime N│   (dynamic) │    │
│  │  │         │  │         │  │         │             │    │
│  │  │SvgoFmt  │  │SvgoFmt  │  │SvgoFmt  │             │    │
│  │  └────┬────┘  └────┬────┘  └────┬────┘             │    │
│  │       │            │            │                  │    │
│  │       └────────────┼────────────┘                  │    │
│  │                    │                               │    │
│  │             async_channel                          │    │
│  └────────────────────┼───────────────────────────────┘    │
│                       │                                    │
└───────────────────────┼────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                      V8 / deno_core                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                 Bundled JavaScript                  │    │
│  │  ┌─────────────┐  ┌─────────────┐                   │    │
│  │  │   SVGO      │  │  Polyfills  │                   │    │
│  │  │  optimize() │  │  (URL,etc)  │                   │    │
│  │  └─────────────┘  └─────────────┘                   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Crate Structure

```tree
dprint-plugin-svgo/
├── Cargo.toml              # Workspace definition
│
├── base/                   # dprint-plugin-deno-base
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module exports
│       ├── runtime.rs      # JsRuntime wrapper
│       ├── channel.rs      # Thread pool management
│       ├── snapshot.rs     # V8 snapshot handling
│       ├── util.rs         # System utilities
│       └── build.rs        # Build-time helpers
│
├── plugin/                 # dprint-plugin-svgo
│   ├── Cargo.toml
│   ├── build.rs            # Snapshot/extensions generation
│   ├── src/
│   │   ├── lib.rs          # Crate root
│   │   ├── handler.rs      # AsyncPluginHandler impl
│   │   ├── formatter.rs    # V8 format execution
│   │   ├── config.rs       # Configuration parsing
│   │   └── main.rs         # Binary entry
│   ├── tests/
│   │   └── tests.rs        # Integration tests
│   └── examples/           # Example usage
│
└── js/                     # JavaScript source
    ├── main.js             # Deno polyfills
    └── node/
        ├── main.ts         # SVGO wrapper
        ├── build.ts        # Bundle script
        └── package.json    # Dependencies
```

## Data Flow Sequence

```flow
Client           Handler          Channel         Runtime           JS
  │                 │                │               │               │
  │  format()       │                │               │               │
  ├────────────────>│                │               │               │
  │                 │ format()       │               │               │
  │                 ├───────────────>│               │               │
  │                 │                │ create_js_runtime (if needed) │
  │                 │                ├──────────────>│               │
  │                 │                │               │ new()         │
  │                 │                │               ├──────────────>│
  │                 │                │               │<──────────────┤
  │                 │                │<──────────────┤               │
  │                 │                │               │               │
  │                 │                │ send(request) │               │
  │                 │                ├──────────────>│               │
  │                 │                │               │ format_text() │
  │                 │                │               ├──────────────>│
  │                 │                │               │  optimize()   │
  │                 │                │               │<──────────────┤
  │                 │                │  recv(result) │               │
  │                 │                │<──────────────┤               │
  │                 │  FormatResult  │               │               │
  │                 │<───────────────┤               │               │
  │  Vec<u8>|None   │                │               │               │
  │<────────────────┤                │               │               │
```

## Memory Management Strategy

```
┌─────────────────────────────────────────┐
│        System Available Memory          │
└─────────────────────┬───────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────┐
│  Memory Check: available > 2.2 * 100MB  │
└─────────────┬───────────────┬───────────┘
              │               │
         ┌────┴────┐     ┌────┴────┐
         │  Yes    │     │   No    │
         └────┬────┘     └────┬────┘
              │               │
              ▼               ▼
    ┌─────────────────┐  ┌────────────────┐
    │ Create Runtime  │  │ Queue Request  │
    │ Increment Total │  │ Wait for Free  │
    └─────────────────┘  └────────────────┘
```

### Auto-Shutdown Logic

```rust
// After 30 seconds idle
if total_runtimes > 1 && pending_runtimes > 1 {
    // Safe to shutdown this runtime
    total_runtimes -= 1;
    pending_runtimes -= 1;
    return;
}
// Otherwise keep alive (last resort runtime)
```

## Configuration Processing

```
┌────────────────┐     ┌────────────────┐     ┌────────────────┐
│   dprint.json  │     │   ConfigKeyMap │     │   SvgoConfig   │
│                │ ──> │                │ ──> │                │
│ "svgo": {...}  │     │ indentWidth    │     │ main: {        │
│                │     │ newLineKind    │     │   js2svg: {    │
└────────────────┘     │ pretty         │     │     indent     │
                       │ multipass      │     │     eol        │
                       │ svg.multipass  │     │     pretty     │
                       └────────────────┘     │   }            │
                                              │   multipass    │
                                              │ }              │
                                              │ extension_     │
                                              │   overrides    │
                                              └────────────────┘
```

## Build Pipeline

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  main.ts     │     │   deno/      │     │  dist/       │
│  + svgo      │ ──> │   esbuild    │ ──> │  main.js     │
│  + polyfills │     │              │     │              │
└──────────────┘     └──────────────┘     └──────────────┘
                                                 │
                                                 ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Cargo build │ <── │  build.rs    │ <── │ Bundled JS   │
│              │     │  snapshot    │     │              │
│  Final       │     │  creation    │     │              │
│  binary      │     │              │     │              │
└──────────────┘     └──────────────┘     └──────────────┘
```

## Extension System

```
┌─────────────────────────────────────────────────────────┐
│                     deno_core Extensions                │
├─────────────────┬─────────────────┬─────────────────────┤
│  deno_webidl    │  deno_console   │    deno_url         │
│  WebIDL types   │  console.log()  │    URL, URLPattern  │
└─────────────────┴─────────────────┴─────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                      main extension                     │
│  globalThis.URL, globalThis.console, etc.               │
└─────────────────────────────────────────────────────────┘
```

## Error Propagation

```
JS Error (SVGO)
      │
      ▼
console.error() + return undefined
      │
      ▼
Result<Option<String>> in Rust
      │
      ▼
Map to Vec<u8> / None
      │
      ▼
FormatResult to dprint
```

## File Matching

```
Plugin advertises: ["svg"]
      │
      ▼
dprint matches files by extension
      │
      ▼
Sends FormatRequest with file_path
      │
      ▼
Plugin extracts extension for overrides:
  "file.svg" -> "svg" -> check "svg.multipass"
```

---

_Architecture documentation for dprint-plugin-svgo_
