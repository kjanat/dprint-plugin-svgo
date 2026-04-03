// Minimal console implementation. Always prints to stderr because stdout
// is reserved for dprint IPC communication.
//
// @ts-ignore: Runs inside deno_core's JsRuntime, not the public Deno API.
// Deno.core is internal/unstable and absent from standard type definitions.
const core = globalThis.Deno.core;

// @ts-ignore X
const print = (...args) => core.print(args.map(String).join(" ") + "\n", true);

// @ts-ignore X
globalThis.console = {
  log: print,
  info: print,
  warn: print,
  error: print,
  debug: print,
};
