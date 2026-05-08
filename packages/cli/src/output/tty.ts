import { spinner } from "@clack/prompts";
import type { Output, OutputOptions, Progress } from "@skilltap/core";
import { ansi, errorLine, infoLine, successLine } from "../ui/format";

export function createTtyOutput(opts: OutputOptions): Output {
  const quiet = opts.quiet ?? false;
  const stdout = opts.stdout ?? process.stdout;
  const stderr = opts.stderr ?? process.stderr;

  return {
    mode: "tty",
    info(msg) {
      if (quiet) return;
      if (opts.stdout) {
        stdout.write(`${msg}\n`);
      } else {
        infoLine(msg);
      }
    },
    warn(msg, hint) {
      stderr.write(`${ansi.yellow("warning")}: ${msg}\n`);
      if (hint) stderr.write(`  ${ansi.dim("hint")}: ${hint}\n`);
    },
    error(msg, hint) {
      if (opts.stderr) {
        stderr.write(`${ansi.red("error")}: ${msg}\n`);
        if (hint) stderr.write(`  ${ansi.dim("hint")}: ${hint}\n`);
      } else {
        errorLine(msg, hint);
      }
    },
    success(msg) {
      if (quiet) return;
      if (opts.stdout) {
        stdout.write(`${ansi.green("✓")} ${msg}\n`);
      } else {
        successLine(msg);
      }
    },
    block(lines, blockOpts) {
      const out = (blockOpts?.stream ?? "stderr") === "stdout" ? stdout : stderr;
      out.write(`${lines.join("\n")}\n`);
    },
    json() {},
    progress(label) {
      return createTtyProgress(label, quiet);
    },
    raw(text) {
      stdout.write(text);
    },
  };
}

function createTtyProgress(label: string, quiet: boolean): Progress {
  if (quiet) return noopProgress();
  const s = spinner();
  s.start(label);
  let active = true;
  return {
    update(msg) {
      if (active) s.message(msg);
    },
    succeed(msg) {
      if (active) {
        s.stop(msg ?? label);
        active = false;
      }
    },
    fail(msg) {
      if (active) {
        s.stop(msg ?? label, 1);
        active = false;
      }
    },
    pause() {
      if (active) {
        s.stop();
        active = false;
      }
    },
    resume() {
      if (!active) {
        s.start(label);
        active = true;
      }
    },
  };
}

function noopProgress(): Progress {
  return {
    update() {},
    succeed() {},
    fail() {},
    pause() {},
    resume() {},
  };
}
