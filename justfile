# https://just.systems

# List available recipes in justfile order.
[group('project')]
default:
    just --justfile {{ justfile() }} --list --unsorted

# Bundle the SVGO wrapper for V8.
[group('plugin')]
build:
    deno task build

# Build and run the full test suite.
[group('plugin')]
test:
    deno task test

# Type-check TypeScript and run cargo clippy.
[group('plugin')]
check:
    deno task check

# Format the repository with dprint.
[group('plugin')]
fmt:
    deno task fmt

# Regenerate the JSON Schema.
[group('maintenance')]
schema:
    deno task schema

# Regenerate the CI workflow YAML.
[group('maintenance')]
ci:
    deno task ci

# Build a release binary and test it with dprint.
[group('plugin')]
local-test:
    deno task local-test

# Check for SVGO updates and prepare a release.
[group('maintenance')]
update:
    deno task update

# Build the Rust crates in debug mode.
[group('rust')]
cargo-build:
    cargo build

# Build the Rust crates in release mode.
[group('rust')]
cargo-release:
    cargo build --release

# Build the site with Bun.
[group('site')]
site-build:
    bun --cwd site build.ts

# Type-check the site with Bun.
[group('site')]
site-typecheck:
    bun --cwd site run typecheck
