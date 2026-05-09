import type { Output, OutputOptions } from "@skilltap/core";

export function createPlainOutput(opts: OutputOptions): Output {
  const quiet = opts.quiet ?? false;
  const stdout = opts.stdout ?? process.stdout;
  const stderr = opts.stderr ?? process.stderr;

  return {
    mode: "plain",
    info(msg) {
      if (quiet) return;
      stdout.write(`${msg}\n`);
    },
    warn(msg, hint) {
      stderr.write(`warning: ${msg}\n`);
      if (hint) stderr.write(`  hint: ${hint}\n`);
    },
    error(msg, hint) {
      stderr.write(`error: ${msg}\n`);
      if (hint) stderr.write(`  hint: ${hint}\n`);
    },
    success(msg) {
      stdout.write(`${msg}\n`);
    },
    block(lines, blockOpts) {
      const out =
        (blockOpts?.stream ?? "stderr") === "stdout" ? stdout : stderr;
      out.write(`${lines.join("\n")}\n`);
    },
    json() {},
    progress(label) {
      if (!quiet) stdout.write(`${label}...\n`);
      return {
        update() {},
        succeed(msg) {
          if (!quiet) stdout.write(`${msg ?? label} done\n`);
        },
        fail(msg) {
          stderr.write(`error: ${msg ?? label} failed\n`);
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
