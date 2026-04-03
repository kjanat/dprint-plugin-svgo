#!/usr/bin/env -S deno run -A

/**
 * CI workflow generator for dprint-plugin-svgo.
 *
 * Generates `.github/workflows/ci.yml` from a programmatic TypeScript
 * definition, ensuring the workflow stays consistent and maintainable.
 *
 * @module
 */

import { stringify } from "yaml";
import $ from "dax";

// --- Configuration ---

const GITHUB_OWNER = "kjanat";
const PLUGIN_NAME = "dprint-plugin-svgo";
const BRANCHES = ["master", "main"];
// Pinned cross-rs/cross commit for aarch64-linux cross-compilation (not published to crates.io)
const CROSS_REV = "f86fd03bb70b4c6802847c18087e21391498b0b4";

type Runner = "macos-latest" | "ubuntu-latest" | "windows-latest";

/** A build target platform with its CI configuration. */
interface Target {
  /** GitHub Actions runner OS. */
  runner: Runner;
  /** Rust target triple (e.g. `x86_64-unknown-linux-gnu`). */
  target: string;
  /** Whether to run `cargo test` for this target. */
  runTests?: boolean;
  /** Whether this target requires cross-compilation via `cross`. */
  cross?: boolean;
  /** Whether to include this target in the reduced PR matrix. */
  runOnPr?: boolean;
}

const targets: Target[] = [
  { runner: "macos-latest", target: "x86_64-apple-darwin", runTests: true },
  {
    runner: "macos-latest",
    target: "aarch64-apple-darwin",
    runTests: true,
    runOnPr: true,
  },
  {
    runner: "windows-latest",
    target: "x86_64-pc-windows-msvc",
    runTests: true,
  },
  {
    runner: "ubuntu-latest",
    target: "x86_64-unknown-linux-gnu",
    runTests: true,
    runOnPr: true,
  },
  { runner: "ubuntu-latest", target: "aarch64-unknown-linux-gnu", cross: true },
];

// --- Derived values ---

function artifactsName(t: Target) {
  return `${t.target}-artifacts`;
}

function zipFileName(t: Target) {
  return `${PLUGIN_NAME}-${t.target}.zip`;
}

function stepId(t: Target) {
  return `pre_release_${t.target.replaceAll("-", "_")}`;
}

// --- Step builders ---
// Each function returns a GitHub Actions step (or array of steps) as a plain
// object that gets serialized to YAML by the `stringify` call at the bottom.

// deno-lint-ignore no-explicit-any
type Step = Record<string, any>;

function checkout(): Step {
  return { uses: "actions/checkout@v6" };
}

function rustToolchain(): Step {
  return { uses: "dsherret/rust-toolchain-file@v1" };
}

function cargoCache(): Step {
  return {
    name: "Cache cargo",
    uses: "Swatinem/rust-cache@v2",
    with: {
      "prefix-key": "v3-${{matrix.config.target}}",
      "save-if": `\${{ ${BRANCHES.map((b) => `github.ref == 'refs/heads/${b}'`).join(" || ")} }}`,
    },
  };
}

function setupDeno(): Step {
  return { uses: "denoland/setup-deno@v2" };
}

function denoBuildJs(): Step {
  return {
    name: "Build JS",
    run: "deno task build",
  };
}

/** Shared setup sequence used by both check and build jobs. */
function setupSteps(): Step[] {
  return [
    checkout(),
    rustToolchain(),
    cargoCache(),
    setupDeno(),
    denoBuildJs(),
  ];
}

function setupRustTarget(): Step {
  const appleTargets = targets
    .filter((t) => t.target.includes("apple"))
    .map((t) => `matrix.config.target == '${t.target}'`);
  return {
    name: "Setup Rust target",
    if: appleTargets.join(" || "),
    run: `rustup target add "\${{matrix.config.target}}"`,
  };
}

/** Pre-build JS bundle and install `cross` for cross-compilation targets. */
function setupCross(): Step[] {
  return [
    {
      name: "Build JS (cross)",
      if: "matrix.config.cross == 'true'",
      run: "deno task build",
    },
    {
      name: "Install cross",
      if: "matrix.config.cross == 'true'",
      run:
        `cargo install cross --locked --git https://github.com/cross-rs/cross --rev ${CROSS_REV}`,
    },
  ];
}

/** Generate a cargo/cross build step with appropriate conditionals for mode and cross-compilation. */
function cargoBuild(mode: "debug" | "release", cross: boolean): Step {
  const isRelease = mode === "release";
  const cmd = cross ? "cross" : "cargo";
  // --all-targets only on debug (catches compile errors in tests/benches/examples).
  // Release builds only need the binary for the artifact zip.
  const flags = cross || isRelease ? "" : " --all-targets";
  const releaseFlag = isRelease ? " --release" : "";
  const crossCond = cross ? "matrix.config.cross == 'true'" : "matrix.config.cross != 'true'";
  const tagCond = isRelease
    ? "startsWith(github.ref, 'refs/tags/')"
    : "!startsWith(github.ref, 'refs/tags/')";
  return {
    name: `Build ${cross ? "cross " : ""}(${isRelease ? "Release" : "Debug"})`,
    if: `${crossCond} && ${tagCond}`,
    run: `${cmd} build --locked${flags} --target "\${{matrix.config.target}}"${releaseFlag}`,
  };
}

function lint(): Step {
  return {
    name: "Lint",
    if:
      "!startsWith(github.ref, 'refs/tags/') && matrix.config.target == 'x86_64-unknown-linux-gnu'",
    run: "cargo clippy",
  };
}

function test(mode: "debug" | "release"): Step {
  const isRelease = mode === "release";
  const tagCond = isRelease
    ? "startsWith(github.ref, 'refs/tags/')"
    : "!startsWith(github.ref, 'refs/tags/')";
  return {
    name: `Test (${isRelease ? "Release" : "Debug"})`,
    if: `matrix.config.run_tests == 'true' && ${tagCond}`,
    run: `cargo test --locked --all-features${isRelease ? " --release" : ""}`,
  };
}

/** Zip the release binary and compute its SHA-256 checksum. Uses PowerShell on Windows, bash elsewhere. */
function preRelease(t: Target): Step {
  const isWindows = t.runner === "windows-latest";
  const zip = zipFileName(t);
  const releaseDir = `target/${t.target}/release`;
  const lines = isWindows
    ? [
      `Compress-Archive -CompressionLevel Optimal -Force -Path ${releaseDir}/${PLUGIN_NAME}.exe -DestinationPath ${releaseDir}/${zip}`,
      `$hash = (Get-FileHash -Algorithm SHA256 ${releaseDir}/${zip}).Hash.ToLower()`,
      `"ZIP_CHECKSUM=$hash" >> $env:GITHUB_OUTPUT`,
    ]
    : [
      `zip -r ${zip} ${PLUGIN_NAME}`,
      `echo "ZIP_CHECKSUM=$(shasum -a 256 ${zip} | awk '{print $1}')" >> "\$GITHUB_OUTPUT"`,
    ];
  const step: Step = {
    name: `Pre-release (${t.target})`,
    id: stepId(t),
    if: `matrix.config.target == '${t.target}' && startsWith(github.ref, 'refs/tags/')`,
    run: lines.join("\n"),
  };
  if (isWindows) {
    step.shell = "pwsh";
  } else {
    step["working-directory"] = releaseDir;
  }
  return step;
}

function uploadArtifact(t: Target): Step {
  return {
    name: `Upload artifacts (${t.target})`,
    if: `matrix.config.target == '${t.target}' && startsWith(github.ref, 'refs/tags/')`,
    uses: "actions/upload-artifact@v7",
    with: {
      name: artifactsName(t),
      path: `target/${t.target}/release/${zipFileName(t)}`,
    },
  };
}

/** Generate JSON schema from Rust types (only on linux-x86_64 release builds). */
const SCHEMA_TARGET = "x86_64-unknown-linux-gnu";

function generateSchema(): Step {
  return {
    name: "Generate schema",
    if: `matrix.config.target == '${SCHEMA_TARGET}' && startsWith(github.ref, 'refs/tags/')`,
    run: "cargo run --locked --features schema --bin generate-schema -- schema.json",
  };
}

function uploadSchemaArtifact(): Step {
  return {
    name: "Upload schema artifact",
    if: `matrix.config.target == '${SCHEMA_TARGET}' && startsWith(github.ref, 'refs/tags/')`,
    uses: "actions/upload-artifact@v7",
    with: {
      name: "schema-artifacts",
      path: "schema.json",
    },
  };
}

// --- Job builders ---

/** Convert target definitions into a GitHub Actions matrix configuration. */
function matrixConfig(items: Target[]) {
  return {
    config: items.map((t) => ({
      os: t.runner,
      run_tests: (t.runTests ?? false).toString(),
      cross: (t.cross ?? false).toString(),
      target: t.target,
    })),
  };
}

/**
 * Create a CI build job with the given targets and event condition.
 * Used for both the PR `check` job (reduced matrix) and the push `build` job (full matrix).
 */
function buildJob(
  items: Target[],
  condition: string,
  opts?: { includeRelease?: boolean },
) {
  const includeRelease = opts?.includeRelease ?? true;
  return {
    name: "${{ matrix.config.target }}",
    if: condition,
    "runs-on": "${{ matrix.config.os }}",
    strategy: { matrix: matrixConfig(items) },
    env: { CARGO_INCREMENTAL: 0, RUST_BACKTRACE: "full" },
    steps: [
      ...setupSteps(),
      setupRustTarget(),
      ...setupCross(),
      cargoBuild("debug", false),
      cargoBuild("release", false),
      cargoBuild("debug", true),
      cargoBuild("release", true),
      lint(),
      test("debug"),
      test("release"),
      ...(includeRelease ? items.map(preRelease) : []),
      ...(includeRelease ? items.map(uploadArtifact) : []),
      ...(includeRelease ? [generateSchema(), uploadSchemaArtifact()] : []),
    ],
  };
}

/** Generate the GitHub Release body with install instructions. */
function releaseBody(): string {
  const tag = "${{ steps.get_tag_version.outputs.TAG_VERSION }}";
  const checksum = "${{ steps.get_plugin_file_checksum.outputs.CHECKSUM }}";
  const version = "${{ steps.get_svgo_version.outputs.SVGO_VERSION }}";
  const pluginUrl =
    `https://github.com/${GITHUB_OWNER}/${PLUGIN_NAME}/releases/download/${tag}/plugin.json@${checksum}`;

  return [
    `SVGO ${version}`,
    "## Install",
    "",
    "Dependencies:",
    "",
    "- Install dprint's CLI >= 0.40.0",
    "",
    "In a dprint configuration file:",
    "",
    `1. Specify the plugin url and checksum in the \`"plugins"\` array or run \`dprint add ${GITHUB_OWNER}/${PLUGIN_NAME}\`:`,
    "",
    "   ```jsonc",
    "   {",
    "     // etc...",
    '     "plugins": [',
    "       // ...add other dprint plugins here...",
    `       "${pluginUrl}"`,
    "     ]",
    "   }",
    "   ```",
    "",
    '2. Add a `"svgo"` configuration property if desired.',
    "",
    "   ```jsonc",
    "   {",
    "     // ...etc...",
    '     "svgo": {',
    '       "multipass": true,',
    '       "pretty": true,',
    '       "indent": 2',
    "     }",
    "   }",
    "   ```",
    "",
  ].join("\n");
}

/** Create the draft release job that downloads artifacts, computes checksums, and publishes. */
function draftReleaseJob() {
  return {
    name: "draft_release",
    if: "startsWith(github.ref, 'refs/tags/')",
    needs: "build",
    "runs-on": "ubuntu-latest",
    permissions: { contents: "write" },
    steps: [
      { name: "Checkout", uses: "actions/checkout@v6" },
      { name: "Download artifacts", uses: "actions/download-artifact@v8" },
      setupDeno(),
      {
        name: "Move downloaded artifacts to root directory",
        run: [
          ...targets.map((t) => `mv ${artifactsName(t)}/${zipFileName(t)} .`),
          "mv schema-artifacts/schema.json .",
        ].join("\n"),
      },
      {
        name: "Output checksums",
        run: targets
          .map((t) =>
            `echo "${zipFileName(t)}: $(shasum -a 256 ${zipFileName(t)} | awk '{print $1}')"`
          )
          .join("\n"),
      },
      {
        name: "Create plugin file",
        run: "deno run --frozen -A scripts/create_plugin_file.ts",
      },
      {
        name: "Get svgo version",
        id: "get_svgo_version",
        run:
          'echo "SVGO_VERSION=$(deno run --frozen --allow-read scripts/output_svgo_version.ts)" >> "$GITHUB_OUTPUT"',
      },
      {
        name: "Get tag version",
        id: "get_tag_version",
        run: 'echo "TAG_VERSION=${GITHUB_REF/refs\\/tags\\//}" >> "$GITHUB_OUTPUT"',
      },
      {
        name: "Get plugin file checksum",
        id: "get_plugin_file_checksum",
        run:
          'echo "CHECKSUM=$(shasum -a 256 plugin.json | awk \'{print $1}\')" >> "$GITHUB_OUTPUT"',
      },
      {
        name: "Release",
        uses: "softprops/action-gh-release@v2",
        env: { GITHUB_TOKEN: "${{ github.token }}" },
        with: {
          draft: true,
          files: [...targets.map(zipFileName), "plugin.json", "schema.json"]
            .join("\n"),
          body: releaseBody(),
        },
      },
    ],
  };
}

// --- Assemble and write ---

const prTargets = targets.filter((t) => t.runOnPr);

const ci = {
  name: "CI",
  on: {
    pull_request: { branches: [...BRANCHES] },
    push: { branches: [...BRANCHES], tags: ["*"] },
  },
  permissions: { contents: "read" },
  concurrency: {
    group: "${{ github.workflow }}-${{ github.event.pull_request.number || github.sha }}",
    "cancel-in-progress": true,
  },
  jobs: {
    check: buildJob(prTargets, "github.event_name == 'pull_request'", {
      includeRelease: false,
    }),
    build: buildJob(targets, "github.event_name == 'push'"),
    draft_release: draftReleaseJob(),
  },
};

let output = "# GENERATED BY ./ci.generate.ts -- DO NOT DIRECTLY EDIT\n\n";
output += stringify(ci, {
  lineWidth: 10_000,
  defaultStringType: "QUOTE_DOUBLE",
  defaultKeyType: "PLAIN",
});

Deno.writeTextFileSync(new URL("./ci.yml", import.meta.url), output);
try {
  await $`dprint fmt`;
} catch {
  // dprint may not be installed; formatting is optional
}
