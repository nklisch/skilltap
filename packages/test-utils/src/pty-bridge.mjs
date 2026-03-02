#!/usr/bin/env node
/**
 * PTY bridge — runs as a Node.js subprocess, controls a PTY using node-pty,
 * and communicates with the Bun test runner via JSON-line streams on
 * stdin/stdout.
 *
 * Protocol (newline-delimited JSON):
 *
 *   stdin  → bridge:  { "type": "send",   "data": "..." }
 *                     { "type": "kill"  }
 *
 *   bridge → stdout:  { "type": "data",   "text": "..." }
 *                     { "type": "exit",   "code": 0     }
 */
import { createRequire } from "module";
import { fileURLToPath } from "url";
import path from "path";

// Resolve node-pty from test-utils' own node_modules (where it was installed)
const __dir = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const ptyPath = path.join(__dir, "..", "node_modules", "@homebridge", "node-pty-prebuilt-multiarch");
const pty = require(ptyPath);

// Bridge config arrives as a single JSON line on argv[2]
const config = JSON.parse(process.argv[2]);
const { cmd, args, cwd, env, cols, rows } = config;

const term = pty.spawn(cmd, args, {
  name: "xterm-color",
  cols: cols ?? 80,
  rows: rows ?? 24,
  cwd,
  env,
});

// Forward PTY output to stdout as JSON lines
term.onData((text) => {
  process.stdout.write(JSON.stringify({ type: "data", text }) + "\n");
});

term.onExit(({ exitCode }) => {
  process.stdout.write(JSON.stringify({ type: "exit", code: exitCode ?? 0 }) + "\n");
  process.exit(0);
});

// Forward stdin commands to the PTY
let stdinBuf = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  stdinBuf += chunk;
  const lines = stdinBuf.split("\n");
  stdinBuf = lines.pop() ?? "";
  for (const line of lines) {
    if (!line.trim()) continue;
    try {
      const msg = JSON.parse(line);
      if (msg.type === "send") term.write(msg.data);
      else if (msg.type === "kill") term.kill();
    } catch {
      // ignore malformed input
    }
  }
});
