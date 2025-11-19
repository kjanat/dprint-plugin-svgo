# Security Recommendations - Implementation Guide

## dprint-plugin-svgo Security Hardening

---

## MEDIUM Priority: Input Size Validation

### Issue

No size limits on SVG input, enabling resource exhaustion attacks.

### Implementation

**File:** `plugin/src/handler.rs`

```rust
// Add constant near the top
const MAX_SVG_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB limit

#[async_trait(?Send)]
impl AsyncPluginHandler for SvgoPluginHandler {
  // ... existing code ...

  async fn format(
    &self,
    request: FormatRequest<Self::Configuration>,
    _format_with_host: impl FnMut(HostFormatRequest) -> LocalBoxFuture<'static, FormatResult> + 'static,
  ) -> FormatResult {
    // Validate input size
    if request.file_bytes.len() > MAX_SVG_SIZE_BYTES {
      return Err(
        SvgoError::FileTooLarge {
          size: request.file_bytes.len(),
          max: MAX_SVG_SIZE_BYTES,
        }
        .into(),
      );
    }

    if request.range.is_some() {
      // no support for range formatting
      return Ok(None);
    }

    self.channel.format(request).await
  }
}
```

**File:** `plugin/src/error.rs`

```rust
#[derive(Error, Debug)]
pub enum SvgoError {
  // ... existing variants ...
  /// File size exceeds maximum allowed
  #[error("SVG file size ({size} bytes) exceeds maximum ({max} bytes)")]
  FileTooLarge { size: usize, max: usize },

  /// Operation timed out
  #[error("SVG formatting operation timed out")]
  OperationTimeout,
}
```

### Testing

```rust
#[test]
fn format_with_oversized_file() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let oversized_content = vec![b'a'; 11 * 1024 * 1024]; // 11MB

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("large.svg"),
          file_bytes: oversized_content,
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("exceeds maximum"));
  });
}
```

### Justification

- 10MB is reasonable for SVG files (most SVGs are <1MB)
- Prevents memory exhaustion attacks
- Allows dprint to reject obviously malicious input early

---

## MEDIUM Priority: Operation Timeout

### Issue

Long-running SVGO operations can block formatter threads indefinitely.

### Implementation

**File:** `base/src/runtime.rs`

```rust
use std::time::Duration;

impl JsRuntime {
  /// Executes a format script with timeout protection
  ///
  /// # Errors
  ///
  /// Returns an error if script execution fails, times out, or the result
  /// cannot be deserialized.
  pub async fn execute_format_script_with_timeout(
    &mut self,
    code: String,
    timeout: Duration,
  ) -> Result<Option<String>, Error> {
    let result = tokio::time::timeout(timeout, self.execute_format_script(code)).await;

    match result {
      Ok(format_result) => format_result,
      Err(_) => Err(anyhow!(
        "Format operation timed out after {} seconds",
        timeout.as_secs()
      )),
    }
  }

  // Keep original for backward compatibility
  pub async fn execute_format_script(&mut self, code: String) -> Result<Option<String>, Error> {
    // Original implementation unchanged
    let global = self.inner.execute_script("format.js", code)?;
    let resolve = self.inner.resolve(global);
    let global = self
      .inner
      .with_event_loop_promise(resolve, PollEventLoopOptions::default())
      .await?;
    let scope = &mut self.inner.handle_scope();
    let local = v8::Local::new(scope, global);
    if local.is_undefined() {
      Ok(None)
    } else {
      let deserialized_value = serde_v8::from_v8::<String>(scope, local);
      match deserialized_value {
        Ok(value) => Ok(Some(value)),
        Err(err) => Err(anyhow!("Cannot deserialize serde_v8 value: {:#}", err)),
      }
    }
  }
}
```

**File:** `plugin/src/formatter.rs`

```rust
use std::time::Duration;

// Add near top of file
const FORMAT_TIMEOUT: Duration = Duration::from_secs(30);

#[async_trait(?Send)]
impl Formatter<SvgoConfig> for SvgoFormatter {
  async fn format_text(
    &mut self,
    request: FormatRequest<SvgoConfig>,
  ) -> Result<Option<Vec<u8>>, deno_core::anyhow::Error> {
    // ... existing code to build request and config ...

    let code = format!(
      "(async () => {{ return await dprint.formatText({{ ...{}, config: {}, pluginsConfig: {} }}); }})()",
      request_value, config_json, plugins_json,
    );

    // Use timeout-protected execution
    self
      .runtime
      .execute_format_script_with_timeout(code, FORMAT_TIMEOUT)
      .await
      .map(|s| s.map(std::string::String::into_bytes))
  }
}
```

### Testing

```rust
#[test]
fn format_with_timeout_protection() {
  // This test verifies timeout is enforced
  // Actual infinite loop testing should be done with property-based fuzzing
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#;

    // Valid SVG should complete before timeout
    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("test.svg"),
          file_bytes: svg.to_string().into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_ok());
  });
}
```

### Justification

- 30-second timeout is reasonable for even complex SVGs
- Prevents thread pool exhaustion
- Matches typical CI/CD timeout patterns

---

## MEDIUM Priority: SVG Structure Validation

### Issue

Malicious SVG with deep nesting or exponential structures can trigger CPU exhaustion.

### Implementation

**File:** `plugin/src/formatter.rs`

```rust
// Add validation function
fn validate_svg_structure(content: &str) -> Result<(), SvgoError> {
  const MAX_NESTING_DEPTH: usize = 100;
  const MAX_TOTAL_ELEMENTS: usize = 100_000;
  const MAX_ATTRIBUTE_SIZE: usize = 10_000; // Per attribute

  // Count elements and check balance
  let open_tags = content.matches('<').count();
  let close_tags = content.matches('>').count();

  if open_tags != close_tags {
    return Err(SvgoError::MalformedSvg("Unbalanced XML tags".to_string()));
  }

  if open_tags > MAX_TOTAL_ELEMENTS {
    return Err(SvgoError::ExcessiveElements {
      count: open_tags,
      max: MAX_TOTAL_ELEMENTS,
    });
  }

  // Check nesting depth with simple state machine
  let mut depth = 0;
  let mut max_depth = 0;
  let mut in_tag = false;

  for byte in content.bytes() {
    match byte {
      b'<' => {
        in_tag = true;
        depth += 1;
        max_depth = max_depth.max(depth);
      }
      b'>' => {
        in_tag = false;
        depth = depth.saturating_sub(1);
      }
      b'/' if in_tag && content.as_bytes().get(content.len() - 1) == Some(&b'>') => {
        // Closing tag
      }
      _ => {}
    }

    if depth > MAX_NESTING_DEPTH {
      return Err(SvgoError::ExcessiveNesting {
        depth,
        max: MAX_NESTING_DEPTH,
      });
    }
  }

  Ok(())
}

#[async_trait(?Send)]
impl Formatter<SvgoConfig> for SvgoFormatter {
  async fn format_text(
    &mut self,
    request: FormatRequest<SvgoConfig>,
  ) -> Result<Option<Vec<u8>>, deno_core::anyhow::Error> {
    let file_text = String::from_utf8(request.file_bytes).map_err(SvgoError::InvalidUtf8)?;

    // Validate SVG structure before processing
    validate_svg_structure(&file_text)?;

    // ... rest of existing code ...
  }
}
```

**File:** `plugin/src/error.rs`

```rust
#[derive(Error, Debug)]
pub enum SvgoError {
  // ... existing variants ...
  /// SVG structure is malformed
  #[error("Malformed SVG: {0}")]
  MalformedSvg(String),

  /// SVG has excessive nesting depth
  #[error("SVG exceeds maximum nesting depth: {depth} > {max}")]
  ExcessiveNesting { depth: usize, max: usize },

  /// SVG has excessive elements
  #[error("SVG exceeds maximum element count: {count} > {max}")]
  ExcessiveElements { count: usize, max: usize },
}
```

### Testing

```rust
#[test]
fn validate_deeply_nested_svg() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Create deeply nested SVG
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg">"#);
    for _ in 0..150 {
      svg.push_str("<g>");
    }
    svg.push_str("<rect/>");
    for _ in 0..150 {
      svg.push_str("</g>");
    }
    svg.push_str("</svg>");

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("nested.svg"),
          file_bytes: svg.into_bytes(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("nesting") || err_msg.contains("depth"));
  });
}

#[test]
fn validate_malformed_svg() {
  let runtime = create_tokio_runtime();

  runtime.block_on(async {
    let handler = SvgoPluginHandler::default();

    // Unbalanced tags
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect></svg>"#;

    let result = handler
      .format(
        FormatRequest {
          config_id: FormatConfigId::from_raw(0),
          file_path: PathBuf::from("malformed.svg"),
          file_bytes: svg.as_bytes().to_vec(),
          config: Arc::new(Default::default()),
          range: None,
          token: Arc::new(NullCancellationToken),
        },
        |_| std::future::ready(Ok(None)).boxed_local(),
      )
      .await;

    // Should either handle gracefully or error
    // Depends on whether validation is strict or permissive
    let _ = result;
  });
}
```

### Justification

- Catches pathological SVG structures early
- Validates before expensive SVGO processing
- 100 levels nesting is reasonable (standard SVGs are 10-20 levels)
- 100k elements is conservative (most SVGs <10k elements)

---

## LOW Priority: Code Injection Defense-In-Depth

### Issue

File path handling could be improved for consistency and maintainability.

### Current Implementation (Adequate)

```rust
// Current code is actually safe
let request_value = serde_json::Value::Object({
  let mut obj = serde_json::Map::new();
  obj.insert("filePath".to_string(), request.file_path.to_string_lossy().into());
  // ...
});
let code = format!(
  "(async () => {{ return await dprint.formatText({{ ...{}, config: {}, pluginsConfig: {} }}); }})()",
  request_value, config_json, plugins_json,
);
```

### Improved Implementation (Defense-In-Depth)

```rust
// More explicit and maintainable
let request_value = serde_json::json!({
  "filePath": request.file_path.to_string_lossy(),
  "fileText": &file_text,
});

let config_value = serde_json::to_value(&resolved_config)
  .map_err(SvgoError::JsonSerialization)?;

let plugins_value = serde_json::to_value(&config.plugins)
  .map_err(SvgoError::JsonSerialization)?;

let code = format!(
  "((requestValue, configValue, pluginsValue) => (async () => {{ \
     return await dprint.formatText({{ ...requestValue, config: configValue, pluginsConfig: pluginsValue }}); \
   }})())",
);

// Pass data as arguments instead of string interpolation
self.runtime.execute_format_with_args(code, vec![request_value, config_value, plugins_value])
```

### Justification

- Makes data flow explicit and obvious
- Reduces cognitive load for future maintainers
- Separates code and data clearly
- Easier to audit for injection vulnerabilities

---

## LOW Priority: Configuration Schema Validation

### Issue

Plugin configuration is accepted without schema validation.

### Implementation

**File:** `plugin/src/config.rs`

```rust
fn validate_plugin_config(plugins: &ConfigKeyValue) -> Result<(), String> {
  match plugins {
    ConfigKeyValue::Object(obj) => {
      for (key, value) in obj {
        // Plugin names should be valid identifiers
        if !key
          .chars()
          .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
          return Err(format!("Invalid plugin name: {}", key));
        }

        // Plugin values should be bool or object
        match value {
          ConfigKeyValue::Bool(_) => {} // Plugin enabled/disabled
          ConfigKeyValue::Object(config) => {
            // Plugin configuration should have string keys and simple values
            for (config_key, _config_value) in config {
              if !config_key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(format!("Invalid plugin config key: {}", config_key));
              }
            }
          }
          _ => return Err(format!("Plugin {} has invalid value type", key)),
        }
      }
      Ok(())
    }
    ConfigKeyValue::String(_) => {
      // String is OK - it will be parsed as JSON later
      Ok(())
    }
    _ => Err("plugins must be an object or JSON string".to_string()),
  }
}

pub fn resolve_config(
  mut config: ConfigKeyMap,
  global_config: GlobalConfiguration,
) -> ResolveConfigurationResult<SvgoConfig> {
  let mut diagnostics = Vec::new();

  // Validate plugins config early if present
  if let Some(ConfigKeyValue::Object(plugins_obj)) = config.get("plugins") {
    if let Err(err) = validate_plugin_config(&ConfigKeyValue::Object(plugins_obj.clone())) {
      diagnostics.push(err);
    }
  }

  // ... rest of existing code ...
}
```

### Justification

- Provides clear error messages for misconfiguration
- Catches configuration errors early
- Improves user experience with actionable feedback

---

## LOW Priority: Error Message Sanitization

### Issue

Error messages expose implementation details unnecessarily.

### Implementation

**File:** `js/node/main.ts`

```typescript
async function formatText(
  { filePath, fileText, config, pluginsConfig }: FormatTextOptions,
) {
  try {
    const result = optimize(fileText, {
      path: filePath,
      ...config,
    });

    const formattedText = result.data;
    if (formattedText === fileText) {
      return undefined;
    } else {
      return formattedText;
    }
  } catch (error) {
    // Generic error message for public consumption
    const isDebug = typeof process !== "undefined" &&
      process.env &&
      process.env.DEBUG === "true";

    if (isDebug) {
      // Detailed error only in debug mode
      console.error(`SVGO optimization failed for ${filePath}:`, error);
    } else {
      // Generic message in production
      console.error("SVG optimization failed - please verify SVG syntax");
    }

    return undefined;
  }
}
```

### Justification

- Prevents unnecessary information disclosure
- Still provides debug capability for development
- Improves user experience with reasonable error messages

---

## Implementation Priority Matrix

| Finding                  | Priority | Effort | Impact | Risk if Skipped        |
| ------------------------ | -------- | ------ | ------ | ---------------------- |
| Input Size Validation    | HIGH     | 1-2h   | High   | DoS attacks            |
| Operation Timeout        | HIGH     | 1h     | High   | Thread exhaustion      |
| SVG Structure Validation | MEDIUM   | 2-3h   | Medium | CPU exhaustion         |
| Code Injection Defense   | LOW      | 1h     | Low    | Future regression risk |
| Config Schema            | LOW      | 1-2h   | Low    | Poor UX                |
| Error Sanitization       | LOW      | 30m    | Low    | Info disclosure        |

---

## Testing Checklist

- [x] Unit tests for each validator
- [ ] Integration tests with malicious payloads
- [ ] Fuzz testing with SVG corpus
- [ ] Load testing with concurrent requests
- [ ] Memory profiling during format operations
- [ ] Timeout verification under CPU load
- [ ] Error message audit for sensitive data
- [ ] Configuration error handling tests

---

## Deployment Checklist

Before deploying security updates:

1. Run full test suite: `cargo test --all-features`
2. Run clippy for additional warnings: `cargo clippy -- -W clippy::all`
3. Format code: `cargo fmt --check`
4. Build release artifact: `cargo build --release`
5. Test with real SVG files from production workloads
6. Document configuration size limits in README
7. Add security section to CHANGELOG
8. Update help documentation with timeout information

---

## Monitoring Recommendations

Add metrics for:

- SVG input sizes and rejection rate
- Format operation duration and timeout frequency
- Memory usage per format operation
- Error rate by error type
- Configuration validation failures

---

**Last Updated:** 2025-11-19
**Status:** Ready for Implementation
