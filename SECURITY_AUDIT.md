# Security Audit Report: dprint-plugin-svgo

## Plugin Crate Comprehensive Security Assessment

**Audit Date:** 2025-11-19
**Auditor:** Security Assessment (DevSecOps)
**Scope:** Plugin crate with focus on input validation, code injection, resource management, and dependency security
**Status:** Generally Secure with Recommendations

---

## Executive Summary

The dprint-plugin-svgo plugin demonstrates solid security fundamentals through proper sandboxing via V8 runtime isolation, input validation at entry points, and memory-aware resource management. No critical vulnerabilities were identified. Five findings are documented below ranging from medium to low severity, primarily involving defense-in-depth improvements and edge case handling.

---

## Detailed Findings

### 1. Code Injection via Unvalidated File Path in JavaScript Context

**Severity:** MEDIUM
**Status:** Implementable | Security Concern
**CVSS Score:** 5.3 (Medium)

#### Description

The `formatter.rs` file constructs JavaScript code by directly embedding file paths into the format script without proper escaping:

```rust
// formatter.rs:64-84
let request_value = serde_json::Value::Object({
  let mut obj = serde_json::Map::new();
  obj.insert(
    "filePath".to_string(),
    request.file_path.to_string_lossy().into(),  // Direct embedding
  );
  // ...
});
let code = format!(
  "(async () => {{ return await dprint.formatText({{ ...{}, config: {}, pluginsConfig: {} }}); }})()",
  request_value, config_json, plugins_json,  // JavaScript injection point
);
```

While file paths are converted to JSON values via `serde_json::to_string()` (which provides JSON escaping), this construction pattern is vulnerable to edge cases.

#### Attack Vector

A specially crafted file path like `test.svg", "injected": "value` could potentially be exploited if the JSON serialization is bypassed or if additional layers of code construction are added in future versions. This is a potential remote attack if file paths come from untrusted sources.

#### Risk Factors

- File paths may come from user input or dynamic sources
- Multiple layers of serialization create cognitive complexity
- Future maintainers might bypass JSON encoding

#### Mitigation Recommendation

Use JSON serialization for ALL dynamic values before string interpolation:

```rust
// RECOMMENDED APPROACH
let request_value = serde_json::json!({
  "filePath": request.file_path.to_string_lossy(),
  "fileText": file_text,
});

let code = format!(
  "(async () => {{ return await dprint.formatText({{}}, config: {}, pluginsConfig: {}); }})()",
  request_value.to_string(),
  config_json,
  plugins_json,
);
```

This ensures ALL values are JSON-escaped consistently and explicitly.

---

### 2. Insufficient Input Size Validation (DoS Vector)

**Severity:** MEDIUM
**Status:** Assessable | Resource Management Gap
**CVSS Score:** 5.0 (Medium)

#### Description

The plugin lacks explicit size limits for input SVG files and JSON configuration. Large inputs can trigger resource exhaustion through:

- Memory allocation in JSON parsing
- V8 runtime memory consumption
- Thread pool saturation

#### Attack Vector

An attacker could submit:

- Extremely large SVG files (GB+ sizes) to exhaust V8 memory
- Deeply nested SVG structures triggering exponential processing
- Large JSON configuration objects with thousands of parameters

Current defenses:

- V8 max memory: 512MB (line 28, formatter.rs)
- Average isolate estimate: 100MB (line 43, handler.rs)
- Memory safety margin: 2.2x (MEMORY_SAFETY_MARGIN in channel.rs)
- Channel capacity: 100 requests (CHANNEL_CAPACITY in channel.rs)

These are reasonable but not enforced at input entry point.

#### Risk Factors

- No validation on `request.file_bytes.len()`
- No validation on configuration object depth
- SVGO multipass mode can trigger multiple passes on large files (line 28, main.ts)
- No timeout on format operations

#### Mitigation Recommendation

Implement input size validation at handler level:

```rust
// In handler.rs format() method
const MAX_SVG_SIZE: usize = 10 * 1024 * 1024; // 10MB

if request.file_bytes.len() > MAX_SVG_SIZE {
  return Err(anyhow::anyhow!(
    "SVG file exceeds maximum size of {} bytes",
    MAX_SVG_SIZE
  ).into());
}
```

Additionally, implement operation timeout:

```rust
// In formatter.rs execute_format_script()
let timeout = std::time::Duration::from_secs(30);
tokio::time::timeout(timeout, self.runtime.execute_format_script(code))
```

---

### 3. Insufficient SVG Content Validation (Indirect Code Execution Risk)

**Severity:** MEDIUM
**Status:** Assessable | Validation Gap
**CVSS Score:** 4.8 (Medium)

#### Description

The plugin passes SVG content directly to SVGO without pre-validation. While SVGO's browser-based version is safer than the Node.js version, malicious SVG can still:

- Trigger parser bugs in SVGO or underlying XML processors
- Cause excessive CPU usage through algorithmic complexity
- Exploit edge cases in optimization logic

#### Attack Vector

Crafted malicious SVG inputs designed to:

- Use billion laughs attack pattern (entity expansion, though unlikely in SVG context)
- Create computationally expensive nested structures
- Exploit specific SVGO plugin vulnerabilities
- Trigger unhandled exceptions in SVGO code

Example:

<!--dprint-ignore-start-->

```xml
<svg xmlns="http://www.w3.org/2000/svg">
  <defs>
    <pattern id="p">
      <pattern id="p1">
      	<pattern id="p2">
          	<!-- deeply nested -->
        </pattern>
      </pattern>
    </pattern>
  </defs>
  <!-- Repeated millions of times -->
  <use href="#p" />
</svg>
```

<!--dprint-ignore-end-->

#### Risk Factors

- SVGO is a complex XML/SVG parser from npm ecosystem
- Browser-based version is sandboxed but may have bugs
- No content-based filtering before processing
- main.ts error handling swallows exceptions without rate limiting

#### Current Defenses

- SVGO error handling: returns undefined on optimization failure (main.ts:40-42)
- V8 sandbox prevents filesystem access
- Memory limits prevent unbounded allocation
- HTTP headers can't be injected in SVG context

#### Mitigation Recommendation

Implement SVG schema validation before SVGO processing:

```rust
// In formatter.rs before calling SVGO
const MAX_NESTING_DEPTH: usize = 50;

fn validate_svg_structure(content: &str) -> Result<(), SvgoError> {
  // Quick structural validation
  let open_tags = content.matches('<').count();
  let close_tags = content.matches('>').count();

  if open_tags != close_tags {
    return Err(SvgoError::MalformedSvg("Unbalanced XML tags".into()));
  }

  // Check nesting depth with simple state machine
  let mut depth = 0;
  for byte in content.bytes() {
    match byte {
      b'<' => depth += 1,
      b'>' => depth = depth.saturating_sub(1),
      _ => {}
    }
    if depth > MAX_NESTING_DEPTH {
      return Err(SvgoError::ExcessiveNesting(depth));
    }
  }
  Ok(())
}
```

---

### 4. Configuration Plugin String Parsing Without Schema Validation

**Severity:** LOW
**Status:** Assessable | Input Validation Edge Case
**CVSS Score:** 3.5 (Low)

#### Description

The `config.rs` file accepts arbitrary JSON strings in the "plugins" configuration key and parses them without schema validation:

```rust
// config.rs:102-107
if key == "plugins"
  && let serde_json::Value::String(s) = &value
  && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s)
{
  value = parsed;
}
```

While the JSON is validated as parseable, there's no schema validation for the resulting structure.

#### Attack Vector

An attacker with configuration access could:

- Inject malformed plugin configurations that cause SVGO runtime errors
- Configure plugins that don't exist (silently ignored by SVGO)
- Create deeply nested plugin configuration structures

The actual risk is low because:

- SVGO silently ignores unknown plugins
- Errors in plugins are caught in the try-catch (main.ts:39-42)
- No RCE vector exists from plugin configuration

#### Risk Factors

- No schema validation of parsed JSON
- Plugins configuration is passed directly to SVGO
- Error handling masks configuration errors

#### Mitigation Recommendation

Implement plugin schema validation:

```rust
// In config.rs
fn validate_plugins_config(plugins: &serde_json::Value) -> Result<(), ConfigError> {
  if let serde_json::Value::Object(obj) = plugins {
    // SVGO plugins should be object with string keys
    for (key, value) in obj {
      if !key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
      {
        return Err(ConfigError::InvalidPluginName(key.clone()));
      }
      // Plugins are either booleans or objects
      if !matches!(
        value,
        serde_json::Value::Bool(_) | serde_json::Value::Object(_)
      ) {
        return Err(ConfigError::InvalidPluginValue);
      }
    }
  }
  Ok(())
}
```

---

### 5. Verbose Error Messages Leaking Implementation Details

**Severity:** LOW
**Status:** Assessable | Information Disclosure
**CVSS Score:** 2.7 (Low)

#### Description

Error messages from SVGO and format operations are exposed directly to users without sanitization:

```typescript
// main.ts:41
console.error(`SVGO error for ${filePath}:`, error);
```

While not exposed to external attackers in typical dprint usage, verbose errors could leak:

- SVGO version and behavior details
- File path information
- Internal exception stack traces

#### Risk Factors

- Errors are logged to stderr
- Stack traces from JavaScript exceptions
- Low priority in typical usage (errors go to dprint's logging)
- dprint handles error sanitization at plugin boundary

#### Attack Vector

Information disclosure to local users or in shared CI/CD logs

#### Mitigation Recommendation

Sanitize error messages:

```typescript
// main.ts - improved error handling
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
    // Generic error message instead of full stack trace
    console.error("SVGO formatting failed - check SVG syntax");
    // Log detailed error only in debug mode
    if (process.env.DEBUG === "true") {
      console.error(`Details - ${filePath}:`, error);
    }
    return undefined;
  }
}
```

---

## Security Strengths

1. **Runtime Sandboxing**: V8 JavaScript runtime is properly sandboxed - no filesystem access to attacker code
2. **UTF-8 Validation**: Input UTF-8 validation at entry point (formatter.rs:70) prevents invalid byte sequences
3. **Memory Management**: Intelligent memory-aware thread pool with 2.2x safety margin prevents unbounded memory allocation
4. **Configuration Parsing**: dprint's configuration framework provides type safety and error tracking
5. **Error Handling**: SVGO errors are caught and handled gracefully without plugin crashes
6. **Dependency Strategy**: Uses official SVGO library (v4.0.0) from npm ecosystem
7. **No External Network Access**: Plugin has no network capabilities
8. **No Shell Execution**: No system command execution or subprocess spawning
9. **Range Formatting Not Supported**: Explicitly rejects range-based formatting (handler.rs:90-92)
10. **Extension-Based Isolation**: Only processes SVG files, not arbitrary binary content

---

## Dependency Security Analysis

### Rust Dependencies (Workspace-Level)

- **deno_core 0.326.0**: Well-maintained Deno runtime base
- **dprint-core 0.67.4**: Stable dprint plugin framework
- **serde/serde_json 1.0.x**: Industry-standard serialization (preserve_order feature enabled for consistency)
- **thiserror 2.x**: Error handling macro crate
- **zstd 0.13.3**: Compression library (build-time only)

All Rust dependencies are from reputable, actively maintained projects.

### JavaScript Dependencies (npm)

- **svgo 4.0.0**: Primary SVGO library - actively maintained
- **buffer, process, url**: Node.js polyfills for browser context (standard approach)

**Recommendation**: Monitor SVGO for security updates in release notes. Current version (4.0.0) is recent and stable.

---

## Additional Security Considerations

### 1. Configuration Injection Risk (Low)

File path used for extension detection (formatter.rs:97-98) uses string slicing without validation:

```rust
let ext = if let Some(index) = file_path.rfind('.') {
  file_path[index + 1..].to_lowercase()
}
```

While safe (only used as dict key in extension_overrides), normalizing to lowercase is good defense-in-depth.

### 2. Thread Safety

- Channel uses `Arc<Mutex<>>` for thread-safe statistics
- V8 runtimes are properly isolated per-thread
- No global mutable state accessible to formatters

### 3. Timeout Handling

**Gap**: No timeout on format operations. A malicious SVG could cause infinite loop in SVGO.

**Recommendation**: Add 30-second timeout in runtime.rs:

```rust
pub async fn execute_format_script_with_timeout(
  &mut self,
  code: String,
  timeout: Duration,
) -> Result<Option<String>, Error> {
  tokio::time::timeout(timeout, self.execute_format_script(code))
    .await
    .map_err(|_| anyhow!("Format operation timed out"))?
}
```

---

## Security Testing Recommendations

1. **Fuzz Testing**: Use SVG fuzzing corpus to test SVGO edge cases
2. **Large File Testing**: Test with 100MB+ SVG files to verify memory limits
3. **Malicious Configuration**: Test with invalid JSON plugin configurations
4. **Nested Structure Testing**: Test deeply nested SVG elements (1000+ levels)
5. **UTF-8 Boundary Testing**: Test various invalid UTF-8 sequences
6. **Memory Profiling**: Profile V8 memory usage under various loads

---

## Compliance & Standards

- **OWASP Top 10 (2021)**:
  - A01 Broken Access Control: Not applicable (no access control)
  - A02 Cryptographic Failures: N/A
  - A03 Injection: Mitigated via JSON serialization
  - A04 Insecure Design: Addressed with sandboxing
  - A06 Vulnerable Components: SVGO is maintained

- **OWASP ASVS**: Applicable requirements implemented for data validation and API security

---

## Remediation Priority

| Priority | Finding                                          | Effort    | Impact                  |
| -------- | ------------------------------------------------ | --------- | ----------------------- |
| High     | Add input size validation (Finding #2)           | 1-2 hours | Prevents DoS attacks    |
| Medium   | Implement operation timeout                      | 1 hour    | Prevents infinite loops |
| Medium   | Add SVG structure validation (Finding #3)        | 2-3 hours | Defense-in-depth        |
| Low      | Improve code injection patterns (Finding #1)     | 1 hour    | Reduces future risk     |
| Low      | Add configuration schema validation (Finding #4) | 1-2 hours | Better error messages   |
| Low      | Sanitize error messages (Finding #5)             | 30 min    | Information disclosure  |

---

## Conclusion

The dprint-plugin-svgo plugin demonstrates solid security fundamentals. The primary attack surface is SVG content processing, which is properly sandboxed in the V8 runtime. Implementing the recommended fixes, particularly input size validation and operation timeouts, will significantly strengthen the security posture.

**Risk Level: LOW**
**Recommendation: APPROVE with conditional implementation of medium-severity findings**

---

## Auditor Notes

- Code quality is high with good error handling
- Architecture properly isolates untrusted SVG processing
- Memory management is proactive and conservative
- Documentation and comments are clear
- Test coverage demonstrates security-conscious design (tests check error paths)

The plugin is suitable for production use with recommended improvements to be implemented in the next maintenance cycle.

---

**Report Generated:** 2025-11-19
**Next Audit Recommended:** After major SVGO updates or security-related changes
