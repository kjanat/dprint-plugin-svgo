# TODO

## High Priority

### Test Gaps

- [ ] Add test for V8 runtime error handling (`SvgoError::Runtime` - hard to trigger externally)
- [ ] Add test for JSON serialization errors (`SvgoError::JsonSerialization` - serde_json::Value always serializes)

### Security (Phase 1 - Mandatory before release)

- [ ] Add input size validation (10MB limit) - prevent DoS
- [ ] Add operation timeout (30 seconds) - prevent CPU exhaustion

## Medium Priority

### Security (Phase 2)

- [ ] Add SVG structure validation (depth/element count limits)
- [ ] Improve JSON serialization consistency for file paths
- [ ] Add configuration schema validation

### Test Improvements

- [x] ~~Reduce test coupling~~ (kept: unit tests appropriately test internals; behavioral tests added)

## Low Priority

### Security (Phase 3)

- [ ] Error message sanitization (avoid leaking internal paths)

### Code Cleanup

- [x] ~~Remove empty `SvgoPluginConfig`~~ (kept: placeholder for future SVGO plugin config)
- [x] ~~`&'static str` for license text~~ (blocked: trait requires `String`)

## Completed

- [x] Create typed error enum with `thiserror`
- [x] Fix `.unwrap()` calls in `formatter.rs`
- [x] Fix `.unwrap()` calls in `config.rs`
- [x] Fix `.unwrap()` call in `handler.rs`
- [x] Fix Clippy warnings (let chains, bool assertions)
- [x] Add unit tests for `resolve_config` (24 tests)
- [x] Add tests for handler trait methods (13 tests)
- [x] Achieve 78% test coverage (100% on plugin source files)
- [x] Add doc comment to `SvgoFormatter` struct
- [x] Use `Path::extension()` for safer extension extraction
- [x] Track cancellation/range formatting TODO (clarified in comments)
- [x] Add tests for config diagnostic messages (3 tests)
- [x] Add concurrent formatting test (5 parallel tasks)
- [x] Add empty/minimal SVG edge case tests (4 tests)
- [x] Add error type display verification test
- [x] Add hidden file extension handling test
- [x] Strengthen behavioral assertions in format tests
- [x] Verify extension override actually affects formatted output
- [x] Add extension override comparison test
