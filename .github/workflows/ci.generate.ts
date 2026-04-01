import { stringify } from "jsr:@std/yaml@^1.0.10";
import $ from "jsr:@david/dax@^0.44.0";

// --- Configuration ---

const PLUGIN_NAME = "dprint-plugin-svgo";
const BRANCHES = ["master", "main"];
// Pinned cross-rs/cross commit for aarch64-linux cross-compilation (not published to crates.io)
const CROSS_REV = "4090beca3cfffa44371a5bba524de3a578aa46c3";

type Runner = "macos-latest" | "ubuntu-latest" | "windows-latest";

interface Target {
  runner: Runner;
  target: string;
  runTests?: boolean;
  cross?: boolean;
  runOnPr?: boolean;
}

const targets: Target[] = [
  { runner: "macos-latest", target: "x86_64-apple-darwin", runTests: true },
  { runner: "macos-latest", target: "aarch64-apple-darwin", runTests: true, runOnPr: true },
  { runner: "windows-latest", target: "x86_64-pc-windows-msvc", runTests: true },
  { runner: "ubuntu-latest", target: "x86_64-unknown-linux-gnu", runTests: true, runOnPr: true },
  { runner: "ubuntu-latest", target: "aarch64-unknown-linux-gnu", cross: true },
];

// --- Derived values ---

function artifactsName(t: Target) {
  return `${t.target}-artifacts`;
}

function zipFileName(t: Target) {
  return `${PLUGIN_NAME}-${t.target}.zip`;
}

function checksumEnvVar(t: Target) {
  return `ZIP_CHECKSUM_${t.target.toUpperCase().replaceAll("-", "_")}`;
}

function stepId(t: Target) {
  return `pre_release_${t.target.replaceAll("-", "_")}`;
}

// --- Step builders ---

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

function denoInstall(): Step {
  return {
    name: "Install dependencies",
    run: "deno install",
    "working-directory": "js/node",
  };
}

function setupSteps(): Step[] {
  return [checkout(), rustToolchain(), cargoCache(), setupDeno(), denoInstall()];
}

function setupRustTarget(): Step {
  const appleTargets = targets
    .filter((t) => t.target.includes("apple"))
    .map((t) => `matrix.config.target == '${t.target}'`);
  return {
    name: "Setup Rust target",
    if: appleTargets.join(" || "),
    run: "rustup target add ${{matrix.config.target}}",
  };
}

function setupCross(): Step[] {
  return [
    {
      name: "Build JS (cross)",
      if: "matrix.config.cross == 'true'",
      run: "deno run -A build.ts",
      "working-directory": "js/node",
    },
    {
      name: "Install cross",
      if: "matrix.config.cross == 'true'",
      run: `cargo install cross --locked --git https://github.com/cross-rs/cross --rev ${CROSS_REV}`,
    },
  ];
}

function cargoBuild(mode: "debug" | "release", cross: boolean): Step {
  const isRelease = mode === "release";
  const cmd = cross ? "cross" : "cargo";
  const flags = cross ? "" : " --all-targets";
  const releaseFlag = isRelease ? " --release" : "";
  const crossCond = cross
    ? "matrix.config.cross == 'true'"
    : "matrix.config.cross != 'true'";
  const tagCond = isRelease
    ? "startsWith(github.ref, 'refs/tags/')"
    : "!startsWith(github.ref, 'refs/tags/')";
  return {
    name: `Build ${cross ? "cross " : ""}(${isRelease ? "Release" : "Debug"})`,
    if: `${crossCond} && ${tagCond}`,
    run: `${cmd} build --locked${flags} --target \${{matrix.config.target}}${releaseFlag}`,
  };
}

function lint(): Step {
  return {
    name: "Lint",
    if: "!startsWith(github.ref, 'refs/tags/') && matrix.config.target == 'x86_64-unknown-linux-gnu'",
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

function preRelease(t: Target): Step {
  const isWindows = t.runner === "windows-latest";
  const zip = zipFileName(t);
  const releaseDir = `target/${t.target}/release`;
  const lines = isWindows
    ? [
      `Compress-Archive -CompressionLevel Optimal -Force -Path ${releaseDir}/${PLUGIN_NAME}.exe -DestinationPath ${releaseDir}/${zip}`,
      `echo "ZIP_CHECKSUM=$(shasum -a 256 ${releaseDir}/${zip} | awk '{print $1}')" >> $GITHUB_OUTPUT`,
    ]
    : [
      `zip -r ${zip} ${PLUGIN_NAME}`,
      `echo "ZIP_CHECKSUM=$(shasum -a 256 ${zip} | awk '{print $1}')" >> $GITHUB_OUTPUT`,
    ];
  const step: Step = {
    name: `Pre-release (${t.target})`,
    id: stepId(t),
    if: `matrix.config.target == '${t.target}' && startsWith(github.ref, 'refs/tags/')`,
    run: lines.join("\n"),
  };
  if (!isWindows) {
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

// --- Job builders ---

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

function buildJob(items: Target[], condition: string) {
  return {
    name: "${{ matrix.config.target }}",
    if: condition,
    "runs-on": "${{ matrix.config.os }}",
    strategy: { matrix: matrixConfig(items) },
    outputs: Object.fromEntries(
      targets.map((t) => [
        checksumEnvVar(t),
        `\${{steps.${stepId(t)}.outputs.ZIP_CHECKSUM}}`,
      ]),
    ),
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
      ...targets.map(preRelease),
      ...targets.map(uploadArtifact),
    ],
  };
}

function releaseBody(): string {
  const tag = "${{ steps.get_tag_version.outputs.TAG_VERSION }}";
  const checksum = "${{ steps.get_plugin_file_checksum.outputs.CHECKSUM }}";
  const version = "${{ steps.get_svgo_version.outputs.SVGO_VERSION }}";
  const pluginUrl = `https://plugins.dprint.dev/svgo-${tag}.json@${checksum}`;

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
    '1. Specify the plugin url and checksum in the `"plugins"` array or run `dprint config add svgo`:',
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

function draftReleaseJob() {
  return {
    name: "draft_release",
    if: "startsWith(github.ref, 'refs/tags/')",
    needs: "build",
    "runs-on": "ubuntu-latest",
    steps: [
      { name: "Checkout", uses: "actions/checkout@v6" },
      { name: "Download artifacts", uses: "actions/download-artifact@v8" },
      setupDeno(),
      {
        name: "Move downloaded artifacts to root directory",
        run: targets.map((t) => `mv ${artifactsName(t)}/${zipFileName(t)} .`).join("\n"),
      },
      {
        name: "Output checksums",
        run: targets
          .map((t) => `echo "${zipFileName(t)}: \${{needs.build.outputs.${checksumEnvVar(t)}}}"`)
          .join("\n"),
      },
      { name: "Create plugin file", run: "deno run -A scripts/create_plugin_file.ts" },
      {
        name: "Get svgo version",
        id: "get_svgo_version",
        run: "echo SVGO_VERSION=$(deno run --allow-read scripts/output_svgo_version.ts) >> $GITHUB_OUTPUT",
      },
      {
        name: "Get tag version",
        id: "get_tag_version",
        run: "echo TAG_VERSION=${GITHUB_REF/refs\\/tags\\//} >> $GITHUB_OUTPUT",
      },
      {
        name: "Get plugin file checksum",
        id: "get_plugin_file_checksum",
        run: "echo \"CHECKSUM=$(shasum -a 256 plugin.json | awk '{print $1}')\" >> $GITHUB_OUTPUT",
      },
      {
        name: "Release",
        uses: "softprops/action-gh-release@v2",
        env: { GITHUB_TOKEN: "${{ secrets.GITHUB_TOKEN }}" },
        with: {
          draft: true,
          files: [...targets.map(zipFileName), "plugin.json"].join("\n"),
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
    pull_request: { branches: BRANCHES },
    push: { branches: BRANCHES, tags: ["*"] },
  },
  concurrency: {
    group: "${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}",
    "cancel-in-progress": true,
  },
  jobs: {
    check: buildJob(prTargets, "github.event_name == 'pull_request'"),
    build: buildJob(targets, "github.event_name == 'push'"),
    draft_release: draftReleaseJob(),
  },
};

let output = "# GENERATED BY ./ci.generate.ts -- DO NOT DIRECTLY EDIT\n\n";
output += stringify(ci, {
  lineWidth: 10_000,
  compatMode: false,
  styles: { "!!str": "double" },
});

Deno.writeTextFileSync(new URL("./ci.yml", import.meta.url), output);
await $`dprint fmt`;
