import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, unlink, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runCompletions(
  args: string[],
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "completions", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

async function runGetCompletions(
  type: string,
  homeDir: string,
  configDir: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "--get-completions", type],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        SKILLTAP_HOME: homeDir,
        XDG_CONFIG_HOME: configDir,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

let homeDir: string;
let configDir: string;

beforeEach(async () => {
  homeDir = await makeTmpDir();
  configDir = await makeTmpDir();
  process.env.SKILLTAP_HOME = homeDir;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  delete process.env.SKILLTAP_HOME;
  delete process.env.XDG_CONFIG_HOME;
  await removeTmpDir(homeDir);
  await removeTmpDir(configDir);
});

// ─── Script generation ────────────────────────────────────────────────────────

describe("completions — bash script", () => {
  test("exits 0 and outputs non-empty script", async () => {
    const { exitCode, stdout } = await runCompletions(
      ["bash"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
  });

  test("script contains completion function", async () => {
    const { stdout } = await runCompletions(["bash"], homeDir, configDir);
    expect(stdout).toContain("_skilltap");
    expect(stdout).toContain("complete -F _skilltap skilltap");
  });

  test("script covers all top-level commands", async () => {
    const { stdout } = await runCompletions(["bash"], homeDir, configDir);
    const commands = [
      "install",
      "remove",
      "list",
      "update",
      "find",
      "link",
      "unlink",
      "info",
      "create",
      "verify",
      "config",
      "tap",
      "doctor",
      "completions",
    ];
    for (const cmd of commands) {
      expect(stdout).toContain(cmd);
    }
  });

  test("script includes --get-completions dynamic calls", async () => {
    const { stdout } = await runCompletions(["bash"], homeDir, configDir);
    expect(stdout).toContain("--get-completions installed-skills");
    expect(stdout).toContain("--get-completions tap-skills");
    expect(stdout).toContain("--get-completions tap-names");
  });
});

describe("completions — zsh script", () => {
  test("exits 0 and outputs non-empty script", async () => {
    const { exitCode, stdout } = await runCompletions(
      ["zsh"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
  });

  test("script starts with #compdef", async () => {
    const { stdout } = await runCompletions(["zsh"], homeDir, configDir);
    expect(stdout.trimStart()).toMatch(/^#compdef skilltap/);
  });

  test("script contains _skilltap function", async () => {
    const { stdout } = await runCompletions(["zsh"], homeDir, configDir);
    expect(stdout).toContain("_skilltap");
    expect(stdout).toContain("_arguments");
    expect(stdout).toContain("_describe");
  });

  test("script covers all top-level commands", async () => {
    const { stdout } = await runCompletions(["zsh"], homeDir, configDir);
    for (const cmd of [
      "install",
      "remove",
      "list",
      "update",
      "find",
      "tap",
      "doctor",
      "completions",
    ]) {
      expect(stdout).toContain(cmd);
    }
  });

  test("script includes dynamic completion calls", async () => {
    const { stdout } = await runCompletions(["zsh"], homeDir, configDir);
    expect(stdout).toContain("--get-completions installed-skills");
    expect(stdout).toContain("--get-completions tap-skills");
    expect(stdout).toContain("--get-completions tap-names");
  });
});

describe("completions — fish script", () => {
  test("exits 0 and outputs non-empty script", async () => {
    const { exitCode, stdout } = await runCompletions(
      ["fish"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.length).toBeGreaterThan(100);
  });

  test("script uses complete -c skilltap pattern", async () => {
    const { stdout } = await runCompletions(["fish"], homeDir, configDir);
    expect(stdout).toContain("complete -c skilltap");
    expect(stdout).toContain("__fish_use_subcommand");
    expect(stdout).toContain("__fish_seen_subcommand_from");
  });

  test("script covers all top-level commands", async () => {
    const { stdout } = await runCompletions(["fish"], homeDir, configDir);
    for (const cmd of [
      "install",
      "remove",
      "list",
      "update",
      "find",
      "tap",
      "doctor",
      "completions",
    ]) {
      expect(stdout).toContain(`-a ${cmd}`);
    }
  });

  test("script includes dynamic completion calls", async () => {
    const { stdout } = await runCompletions(["fish"], homeDir, configDir);
    expect(stdout).toContain("--get-completions installed-skills");
    expect(stdout).toContain("--get-completions tap-skills");
    expect(stdout).toContain("--get-completions tap-names");
  });
});

describe("completions — unknown shell", () => {
  test("exits 1 for unknown shell", async () => {
    const { exitCode, stderr } = await runCompletions(
      ["powershell"],
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("powershell");
  });
});

// ─── --get-completions handler ────────────────────────────────────────────────

describe("--get-completions — empty state", () => {
  test("installed-skills: empty output on fresh install", async () => {
    const { exitCode, stdout } = await runGetCompletions(
      "installed-skills",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });

  test("linked-skills: empty output on fresh install", async () => {
    const { exitCode, stdout } = await runGetCompletions(
      "linked-skills",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });

  test("tap-skills: empty output when no taps", async () => {
    const { exitCode, stdout } = await runGetCompletions(
      "tap-skills",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });

  test("tap-names: empty output when no taps configured", async () => {
    const { exitCode, stdout } = await runGetCompletions(
      "tap-names",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });

  test("unknown type: exits 0 with empty output", async () => {
    const { exitCode, stdout } = await runGetCompletions(
      "unknown-type",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toBe("");
  });
});

describe("--get-completions — with state", () => {
  test("installed-skills: returns skill names", async () => {
    // Write an installed.json with two skills
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "my-skill",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          {
            name: "other-skill",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "project",
            path: null,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ],
      }),
    );

    const { exitCode, stdout } = await runGetCompletions(
      "installed-skills",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    const names = stdout.trim().split("\n");
    expect(names).toContain("my-skill");
    expect(names).toContain("other-skill");
  });

  test("linked-skills: returns only linked skill names", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "installed.json"),
      JSON.stringify({
        version: 1,
        skills: [
          {
            name: "global-skill",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
          {
            name: "linked-skill",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "linked",
            path: null,
            tap: null,
            also: [],
            installedAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
          },
        ],
      }),
    );

    const { exitCode, stdout } = await runGetCompletions(
      "linked-skills",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    const names = stdout.trim().split("\n");
    expect(names).toContain("linked-skill");
    expect(names).not.toContain("global-skill");
  });

  test("tap-names: returns configured tap names", async () => {
    const skilltapDir = join(configDir, "skilltap");
    await mkdir(skilltapDir, { recursive: true });
    await writeFile(
      join(skilltapDir, "config.toml"),
      '[defaults]\nalso = []\nyes = false\nscope = ""\n[security]\nscan = "static"\non_warn = "prompt"\nrequire_scan = false\nagent = ""\nthreshold = 5\nmax_size = 51200\nollama_model = ""\n["agent-mode"]\nenabled = false\nscope = "project"\n[[taps]]\nname = "core"\nurl = "https://github.com/skilltap/taps"\ntype = "git"\n[[taps]]\nname = "my-tap"\nurl = "https://github.com/user/my-tap"\ntype = "git"\n',
    );

    const { exitCode, stdout } = await runGetCompletions(
      "tap-names",
      homeDir,
      configDir,
    );
    expect(exitCode).toBe(0);
    const names = stdout.trim().split("\n");
    expect(names).toContain("core");
    expect(names).toContain("my-tap");
  });
});

// ─── --install flag ───────────────────────────────────────────────────────────

describe("completions --install", () => {
  const home = homedir();

  test("--install bash writes file and exits 0", async () => {
    const expectedPath = join(
      home,
      ".local",
      "share",
      "bash-completion",
      "completions",
      "skilltap",
    );
    try {
      const { exitCode, stdout } = await runCompletions(
        ["bash", "--install"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Wrote completions to");
      expect(stdout).toContain("bash-completion");

      const content = await Bun.file(expectedPath).text().catch(() => null);
      expect(content).not.toBeNull();
      expect(content).toContain("_skilltap");
    } finally {
      await unlink(expectedPath).catch(() => {});
    }
  });

  test("--install zsh writes file to ~/.zfunc/_skilltap", async () => {
    const expectedPath = join(home, ".zfunc", "_skilltap");
    try {
      const { exitCode, stdout } = await runCompletions(
        ["zsh", "--install"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Wrote completions to");
      expect(stdout).toContain(".zfunc");

      const content = await Bun.file(expectedPath).text().catch(() => null);
      expect(content).not.toBeNull();
      expect(content).toContain("#compdef skilltap");
    } finally {
      await unlink(expectedPath).catch(() => {});
    }
  });

  test("--install fish writes file to ~/.config/fish/completions/", async () => {
    const expectedPath = join(
      home,
      ".config",
      "fish",
      "completions",
      "skilltap.fish",
    );
    try {
      const { exitCode, stdout } = await runCompletions(
        ["fish", "--install"],
        homeDir,
        configDir,
      );
      expect(exitCode).toBe(0);
      expect(stdout).toContain("Wrote completions to");
      expect(stdout).toContain("fish");

      const content = await Bun.file(expectedPath).text().catch(() => null);
      expect(content).not.toBeNull();
      expect(content).toContain("complete -c skilltap");
    } finally {
      await unlink(expectedPath).catch(() => {});
    }
  });
});
