import type { Output, OutputOptions } from "@skilltap/core";

export function createJsonOutput(opts: OutputOptions): Output {
  const stdout = opts.stdout ?? process.stdout;

  return {
    mode: "json",
    info() {},
    warn(msg, hint) {
      stdout.write(`${JSON.stringify({ kind: "warn", message: msg, hint })}\n`);
    },
    error(msg, hint) {
      stdout.write(
        `${JSON.stringify({ kind: "error", message: msg, hint })}\n`,
      );
    },
    success() {},
    block() {},
    json(event) {
      stdout.write(`${JSON.stringify(event)}\n`);
    },
    progress(label) {
      stdout.write(`${JSON.stringify({ kind: "progress:start", label })}\n`);
      return {
        update(msg) {
          stdout.write(
            `${JSON.stringify({ kind: "progress:update", label, message: msg })}\n`,
          );
        },
        succeed(msg) {
          stdout.write(
            `${JSON.stringify({ kind: "progress:done", label, message: msg })}\n`,
          );
        },
        fail(msg) {
          stdout.write(
            `${JSON.stringify({ kind: "progress:fail", label, message: msg })}\n`,
          );
        },
        pause() {},
        resume() {},
      };
    },
    raw(text) {
      stdout.write(text);
    },
  };
}
