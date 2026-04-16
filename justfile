# https://just.systems

# List available recipes in justfile order.
[default]
[group('project')]
default:
    @just --justfile {{ justfile() }} --list --unsorted

# Run the standard verification suite.
[group('project')]
verify: fmt check test site-typecheck site-build

# Type-check the Deno scripts.
[private]
check-deno:
    deno check --frozen --check-js scripts/create_plugin_file.ts scripts/generate_schema.ts scripts/lib.ts scripts/local_test.ts scripts/output_local_plugin_ref.ts scripts/output_svgo_version.ts scripts/update.ts .github/workflows/ci.generate.ts

# Lint the Rust crates with clippy.
[private]
check-clippy:
    cargo clippy --all-targets -- -D warnings

# Lint the Rust crates in CI.
[private]
ci-lint: check-clippy

# Bundle the SVGO wrapper for V8.
[group('plugin')]
build:
    deno task --frozen bundle:runtime

# Build and run the full test suite.
[group('plugin')]
test: build
    cargo test --all-features

# Check sample SVG fixtures with local plugin config.
[group('plugin')]
samples-check:
    rm -rf samples-tmp && cp -R samples samples-tmp && trap 'rm -rf samples-tmp' EXIT && plugin_ref=$(deno run --frozen --allow-read scripts/output_local_plugin_ref.ts) && dprint check -c=.dprint.local.jsonc --config-discovery=false --plugins "$plugin_ref"

# Format sample SVG fixtures in a persistent temp copy.
[group('plugin')]
samples-fmt:
    rm -rf samples-tmp && cp -R samples samples-tmp && plugin_ref=$(deno run --frozen --allow-read scripts/output_local_plugin_ref.ts) && exit_code=0; dprint fmt -c=.dprint.local.jsonc --config-discovery=false --plugins "$plugin_ref" || exit_code=$?; printf "*\n" > samples-tmp/.gitignore; exit $exit_code

# Build a locked debug target in CI.
[private]
ci-build-debug target:
    cargo build --locked --all-targets --target {{ target }}

# Build a locked release target in CI.
[private]
ci-build-release target:
    cargo build --locked --target {{ target }} --release

# Build a locked cross-compiled debug target in CI.
[private]
ci-cross-build-debug target:
    cross build --locked --target {{ target }}

# Build a locked cross-compiled release target in CI.
[private]
ci-cross-build-release target:
    cross build --locked --target {{ target }} --release

# Run the locked debug test suite in CI.
[private]
ci-test-debug:
    cargo test --locked --all-features

# Run the locked release test suite in CI.
[private]
ci-test-release:
    cargo test --locked --all-features --release

# Type-check TypeScript and run cargo clippy.
[group('plugin')]
[parallel]
check: check-deno check-clippy

# Format the repository with dprint.
[group('plugin')]
fmt:
    dprint fmt

# Generate site/schema.json for site builds.
[group('maintenance')]
schema:
    deno run --frozen -A scripts/generate_schema.ts site/schema.json

# Run the pre-release checks.
[group('maintenance')]
release-check: verify ci local-test

# Regenerate the CI workflow YAML.
[group('maintenance')]
ci:
    deno run --frozen -A .github/workflows/ci.generate.ts

# Build a release binary and format a disposable workspace with dprint.
[group('plugin')]
local-test:
    deno run --frozen -A scripts/local_test.ts

# Generate the release plugin manifest in CI.
[private]
ci-create-plugin-file:
    deno run --frozen -A scripts/create_plugin_file.ts

# Verify committed schema and site build in CI.
[private]
ci-verify-schema-site: site-typecheck site-build

# Install the site dependencies.
[group('site')]
[working-directory('site')]
site-install:
    bun install

# Print the resolved SVGO version in CI.
[private]
ci-output-svgo-version:
    deno run --frozen --allow-read scripts/output_svgo_version.ts

# Check for SVGO updates and prepare a release.
[confirm("Run update and prepare a release?")]
[group('maintenance')]
update:
    deno run -A scripts/update.ts

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
[working-directory('site')]
site-build: site-install site-schema
    bun run build

# Type-check the site with Bun.
[group('site')]
[working-directory('site')]
site-typecheck: site-install site-schema
    bun run typecheck

# Generate site schema from current repository state.
[private]
site-schema:
    deno run --frozen -A scripts/generate_schema.ts site/schema.json
