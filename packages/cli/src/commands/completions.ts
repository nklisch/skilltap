import { mkdir, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { defineCommand } from "citty";
import type { Shell } from "../completions/generate";
import { generateCompletions } from "../completions/generate";
import { ansi } from "../ui/format";

async function patchZshrc(home: string): Promise<string> {
  const zshrcPath = join(home, ".zshrc");
  let content = "";
  try {
    content = await Bun.file(zshrcPath).text();
  } catch {}
  if (content.includes(".zfunc")) {
    return "  Restart your shell to enable completions.";
  }
  const setup =
    "\n# skilltap completions\nfpath=(~/.zfunc $fpath)\nautoload -Uz compinit && compinit\n";
  try {
    await writeFile(zshrcPath, content + setup);
    return "  Added fpath setup to ~/.zshrc\n  Restart your shell to enable completions.";
  } catch {
    return "  Add to ~/.zshrc (if not already present):\n    fpath=(~/.zfunc $fpath)\n    autoload -Uz compinit && compinit\n  Then restart your shell.";
  }
}

export default defineCommand({
  meta: {
    name: "completions",
    description: "Generate shell completion script",
  },
  args: {
    shell: {
      type: "positional",
      description: "Shell type: bash, zsh, or fish",
      required: true,
    },
    install: {
      type: "boolean",
      description: "Write completions to the shell-standard location",
      default: false,
    },
  },
  async run({ args }) {
    const shell = args.shell as string;

    if (shell !== "bash" && shell !== "zsh" && shell !== "fish") {
      process.stderr.write(
        `Error: Unknown shell '${shell}'. Valid values: bash, zsh, fish\n`,
      );
      process.exit(1);
    }

    const script = generateCompletions(shell as Shell);

    if (!args.install) {
      process.stdout.write(`${script}\n`);
      return;
    }

    const home = process.env.HOME ?? homedir();

    // Warn if the specified shell doesn't match the running shell
    const currentShell = (process.env.SHELL ?? "").split("/").pop() ?? "";
    if (
      currentShell &&
      currentShell !== shell &&
      (["bash", "zsh", "fish"] as string[]).includes(currentShell)
    ) {
      process.stderr.write(
        `${ansi.yellow("Note:")} $SHELL is ${currentShell} — did you mean: skilltap completions ${currentShell} --install?\n`,
      );
    }

    let targetPath: string;
    let instructions: string;

    switch (shell) {
      case "bash":
        targetPath = join(
          home,
          ".local",
          "share",
          "bash-completion",
          "completions",
          "skilltap",
        );
        instructions = `  Restart your shell or run:\n    source ${targetPath.replace(home, "~")}`;
        break;
      case "zsh":
        targetPath = join(home, ".zfunc", "_skilltap");
        instructions = await patchZshrc(home);
        break;
      case "fish":
        targetPath = join(
          home,
          ".config",
          "fish",
          "completions",
          "skilltap.fish",
        );
        instructions = `  Completions are available immediately in new fish sessions.`;
        break;
    }

    await mkdir(dirname(targetPath), { recursive: true });
    await writeFile(targetPath, `${script}\n`);

    const displayPath = targetPath.replace(home, "~");
    process.stdout.write(
      `${ansi.green("✓")} Wrote completions to ${displayPath}\n${instructions}\n`,
    );
  },
});
