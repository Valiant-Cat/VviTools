/**
 * VviTools bundled builtin plugin entry.
 *
 * This file is intentionally not executed by Node. The manifest marks the
 * plugin as `runtime: "builtin"`, so VviTools loads this directory as a
 * first-party bundled plugin and dispatches commands to the Rust host bridge.
 *
 * Host bridge: host.clipboard
 */

export type BuiltinCommand = "clipboard.open";

export type BuiltinRequest = {
  jsonrpc: "2.0";
  method: "run";
  params: {
    command: BuiltinCommand;
    query: string;
    context: Record<string, unknown>;
  };
};

export type BuiltinResult =
  | { type: "text"; text: string }
  | { type: "error"; message: string };

export const plugin = {
  id: "system-clipboard",
  bridge: "host.clipboard",
  commands: {
    "clipboard.open": {
      hostCommand: "clipboard.open",
      description: "Open the independent clipboard panel.",
    },
  },
} as const;

export function run(request: BuiltinRequest): BuiltinResult {
  if (request.params.command !== "clipboard.open") {
    return { type: "error", message: `Unknown command: ${request.params.command}` };
  }

  return { type: "text", text: "Use Alt + V to open the system clipboard panel." };
}
