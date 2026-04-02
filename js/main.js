// Minimal console implementation - always prints to stderr because
// stdout is used for dprint IPC communication.
const core = globalThis.Deno.core;
const print = (msg) => core.print(msg + "\n", true);
globalThis.console = {
  log: print,
  info: print,
  warn: print,
  error: print,
  debug: print,
};
