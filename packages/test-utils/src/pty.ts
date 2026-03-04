import { join } from "node:path";

/**
 * Named key sequences for @clack/prompts navigation.
 * Use with sendKey() in interactive tests.
 */
export const Keys = {
  UP: "\x1b[A",
  DOWN: "\x1b[B",
  LEFT: "\x1b[D",
  RIGHT: "\x1b[C",
  ENTER: "\r",
  SPACE: " ",
  CTRL_C: "\x03",
  BACKSPACE: "\x7f",
  TAB: "\t",
} as const;

export type Key = keyof typeof Keys;

// Matches CSI sequences (\x1b[...X), OSC sequences (\x1b]...\x07), bare \x1b
const ANSI_RE = /\x1b(?:\[[0-9;?]*[a-zA-Z]|\][^\x07]*\x07|[^[\]])/g;

export function stripAnsi(str: string): string {
  return str.replace(ANSI_RE, "");
}

export interface InteractiveOpts {
  cwd?: string;
  env?: Record<string, string>;
  cols?: number;
  rows?: number;
  /** Default timeout for waitForText() calls, in ms. Default: 30000 */
  timeout?: number;
}

export interface InteractiveSession {
  /**
   * Wait until ANSI-stripped output contains the string or matches the regex.
   * On timeout, rejects with a message showing current output for debugging.
   */
  waitForText(match: string | RegExp, timeoutMs?: number): Promise<string>;
  /** Send raw bytes (text, escape sequences, etc.) to the terminal. */
  send(data: string): void;
  /** Send a named key (ENTER, UP, DOWN, SPACE, CTRL_C, etc.). */
  sendKey(key: Key): void;
  /** All output collected so far, with ANSI codes intact. */
  rawOutput(): string;
  /** All output collected so far, ANSI codes stripped. */
  output(): string;
  /**
   * Wait for the process to exit. Returns exit code and final output.
   * Kills the process if it hasn't exited within timeoutMs (default 15s).
   */
  finish(timeoutMs?: number): Promise<{ exitCode: number; output: string }>;
  /** Kill the terminal process immediately. */
  kill(): void;
}

// Path to the Node.js bridge script that owns the node-pty instance.
// We use a real Node.js subprocess to avoid Bun's N-API event-loop
// integration issues that prevent node-pty onData callbacks from firing.
const BRIDGE = join(import.meta.dir, "pty-bridge.mjs");

/**
 * Spawn a CLI command in a real PTY so that interactive prompts work.
 *
 * Internally, a Node.js bridge process owns the PTY via node-pty.
 * The Bun test communicates with the bridge over JSON-line pipes, avoiding
 * the Bun N-API event-loop mismatch that prevents onData callbacks from
 * firing when node-pty is imported directly into a Bun test.
 *
 * @example
 * const session = await runInteractive(
 *   ["bun", "run", "--bun", "src/index.ts", "install", repoPath],
 *   { cwd: CLI_DIR, env: { SKILLTAP_HOME: homeDir, XDG_CONFIG_HOME: configDir } },
 * );
 *
 * // Scope prompt — accept default (Global) by pressing Enter
 * await session.waitForText("Install to:");
 * session.sendKey("ENTER");
 *
 * // Confirm prompt — initialValue:true so Enter accepts
 * await session.waitForText("Install");
 * session.sendKey("ENTER");
 *
 * const { exitCode, output } = await session.finish();
 * expect(exitCode).toBe(0);
 */
export async function runInteractive(
  cmd: [string, ...string[]],
  opts: InteractiveOpts = {},
): Promise<InteractiveSession> {
  const {
    cwd = process.cwd(),
    env = {},
    cols = 80,
    rows = 24,
    timeout: defaultTimeout = 30_000,
  } = opts;

  const [file, ...args] = cmd;

  const bridgeConfig = JSON.stringify({
    cmd: file,
    args,
    cwd,
    env: { ...process.env, ...env },
    cols,
    rows,
  });

  // Spawn the Node.js bridge — it owns the PTY and relays data via JSON lines
  const bridge = Bun.spawn(["node", BRIDGE, bridgeConfig], {
    stdin: "pipe",
    stdout: "pipe",
    stderr: "inherit", // surface bridge errors directly
  });

  let rawBuf = "";
  let exitCode: number | null = null;
  let exitResolve!: (code: number) => void;
  const exitPromise = new Promise<number>((resolve) => {
    exitResolve = resolve;
  });

  // Read JSON lines from the bridge's stdout
  async function readLoop() {
    const reader = bridge.stdout.getReader();
    let pending = "";
    const decoder = new TextDecoder();

    try {
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        pending += decoder.decode(value, { stream: true });
        const lines = pending.split("\n");
        pending = lines.pop() ?? "";
        for (const line of lines) {
          if (!line.trim()) continue;
          try {
            const msg = JSON.parse(line) as
              | { type: "data"; text: string }
              | { type: "exit"; code: number };
            if (msg.type === "data") {
              rawBuf += msg.text;
            } else if (msg.type === "exit") {
              exitCode = msg.code;
              exitResolve(msg.code);
            }
          } catch {
            // ignore malformed JSON from bridge
          }
        }
      }
    } catch {
      // bridge stdout closed
    }
  }

  // Start the read loop (non-blocking; drives the event loop)
  readLoop();

  function sendToBridge(msg: object) {
    const line = JSON.stringify(msg) + "\n";
    bridge.stdin.write(line);
    bridge.stdin.flush();
  }

  function send(data: string) {
    sendToBridge({ type: "send", data });
  }

  function sendKey(key: Key) {
    send(Keys[key]);
  }

  function rawOutput() {
    return rawBuf;
  }

  function output() {
    return stripAnsi(rawBuf);
  }

  function waitForText(
    match: string | RegExp,
    timeoutMs = defaultTimeout,
  ): Promise<string> {
    return new Promise((resolve, reject) => {
      const deadline = Date.now() + timeoutMs;

      const check = () => {
        const plain = stripAnsi(rawBuf);
        const found =
          typeof match === "string" ? plain.includes(match) : match.test(plain);

        if (found) {
          resolve(plain);
          return;
        }

        if (Date.now() >= deadline) {
          reject(
            new Error(
              `waitForText timed out after ${timeoutMs}ms\n` +
                `Waiting for: ${JSON.stringify(match.toString())}\n` +
                `Output so far:\n${stripAnsi(rawBuf)}`,
            ),
          );
          return;
        }

        setTimeout(check, 50);
      };

      check();
    });
  }

  async function finish(
    timeoutMs = 15_000,
  ): Promise<{ exitCode: number; output: string }> {
    const killTimer = setTimeout(() => {
      sendToBridge({ type: "kill" });
      bridge.kill();
    }, timeoutMs);

    const code = await exitPromise;
    clearTimeout(killTimer);

    // Allow the read loop to process any trailing data lines
    await new Promise((resolve) => setTimeout(resolve, 100));

    bridge.kill(); // ensure the bridge process is cleaned up
    return { exitCode: code, output: stripAnsi(rawBuf) };
  }

  function kill() {
    sendToBridge({ type: "kill" });
    bridge.kill();
  }

  return { waitForText, send, sendKey, rawOutput, output, finish, kill };
}
