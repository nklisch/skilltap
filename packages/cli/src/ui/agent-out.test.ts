import { describe, expect, test } from "bun:test";
import type { SemanticWarning, StaticWarning } from "@skilltap/core";

// We test by running these functions in a subprocess and capturing output,
// since they write directly to process.stdout/stderr.

const CLI_DIR = `${import.meta.dir}/../..`;
const ANSI_RE = /\x1b\[/;

async function run(
  code: string,
): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const proc = Bun.spawn(
    [
      "bun",
      "--eval",
      `import { ${code.split("(")[0]} } from "./src/ui/agent-out";\n${code}`,
    ],
    { cwd: CLI_DIR, stdout: "pipe", stderr: "pipe" },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { stdout, stderr, exitCode };
}

describe("agentSuccess", () => {
  test("formats with ref", async () => {
    const { stdout } = await run(
      'agentSuccess("commit-helper", "~/.agents/skills/commit-helper/", "v1.2.0")',
    );
    expect(stdout).toBe(
      "OK: Installed commit-helper → ~/.agents/skills/commit-helper/ (v1.2.0)\n",
    );
    expect(stdout).not.toMatch(ANSI_RE);
  });

  test("formats without ref", async () => {
    const { stdout } = await run(
      'agentSuccess("my-skill", "/path/to/skill/", null)',
    );
    expect(stdout).toBe("OK: Installed my-skill → /path/to/skill/\n");
  });
});

describe("agentUpdated", () => {
  test("formats with refs", async () => {
    const { stdout } = await run(
      'agentUpdated("commit-helper", "v1.0.0", "v1.1.0")',
    );
    expect(stdout).toBe("OK: Updated commit-helper (v1.0.0 → v1.1.0)\n");
    expect(stdout).not.toMatch(ANSI_RE);
  });

  test("formats without refs", async () => {
    const { stdout } = await run('agentUpdated("commit-helper")');
    expect(stdout).toBe("OK: Updated commit-helper\n");
  });
});

describe("agentSkip", () => {
  test("formats correctly", async () => {
    const { stdout } = await run(
      'agentSkip("commit-helper", "is already installed.")',
    );
    expect(stdout).toBe("SKIP: commit-helper is already installed.\n");
    expect(stdout).not.toMatch(ANSI_RE);
  });
});

describe("agentError", () => {
  test("writes to stderr", async () => {
    const { stderr } = await run(
      'agentError("Repository not found: https://example.com/bad-url.git")',
    );
    expect(stderr).toBe(
      "ERROR: Repository not found: https://example.com/bad-url.git\n",
    );
    expect(stderr).not.toMatch(ANSI_RE);
  });
});

describe("agentUpToDate", () => {
  test("formats correctly", async () => {
    const { stdout } = await run('agentUpToDate("commit-helper")');
    expect(stdout).toBe("OK: commit-helper is already up to date.\n");
    expect(stdout).not.toMatch(ANSI_RE);
  });
});

describe("agent-out — snapshot stability", () => {
  test("agentSuccess full output", async () => {
    const { stdout } = await run(
      'agentSuccess("my-skill", "~/.agents/skills/my-skill/", "v1.0.0")',
    );
    expect(stdout).toMatchSnapshot();
  });

  test("agentUpdated full output", async () => {
    const { stdout } = await run(
      'agentUpdated("my-skill", "abc1234", "def5678")',
    );
    expect(stdout).toMatchSnapshot();
  });

  test("agentSkip full output", async () => {
    const { stdout } = await run('agentSkip("my-skill", "is already installed.")');
    expect(stdout).toMatchSnapshot();
  });

  test("agentError full output", async () => {
    const { stderr } = await run('agentError("something went wrong")');
    expect(stderr).toMatchSnapshot();
    expect(stderr.endsWith("\n")).toBe(true);
  });

  test("agentUpToDate full output", async () => {
    const { stdout } = await run('agentUpToDate("my-skill")');
    expect(stdout).toMatchSnapshot();
  });

  test("agentSecurityBlock full output", async () => {
    const { stderr } = await run(`
      agentSecurityBlock(
        [{ file: "SKILL.md", line: 5, category: "Invisible Unicode", raw: "x" }],
        []
      )
    `);
    expect(stderr).toMatchSnapshot();
    expect(stderr).toContain("SECURITY ISSUE FOUND");
    expect(stderr).toContain("DO NOT install");
    expect(stderr).not.toMatch(/\x1b\[/);
  });
});

describe("agentSecurityBlock", () => {
  test("formats static warnings", async () => {
    const { stderr } = await run(`
      agentSecurityBlock(
        [
          { file: "SKILL.md", line: 14, category: "Invisible Unicode (3 zero-width chars)", raw: "test", visible: "test" },
          { file: "SKILL.md", line: 8, category: "Hidden HTML comment", raw: "test" },
        ],
        []
      )
    `);
    expect(stderr).toContain("SECURITY ISSUE FOUND — INSTALLATION BLOCKED");
    expect(stderr).toContain("DO NOT install this skill");
    expect(stderr).toContain("SKILL.md L14: Invisible Unicode");
    expect(stderr).toContain("SKILL.md L8: Hidden HTML comment");
    expect(stderr).toContain("User action required");
    expect(stderr).not.toMatch(ANSI_RE);
  });

  test("formats semantic warnings", async () => {
    const { stderr } = await run(`
      agentSecurityBlock(
        [],
        [
          { file: "SKILL.md", lineRange: [12, 18], chunkIndex: 3, score: 8, reason: "Requests exfiltration of SSH key", raw: "test" },
        ]
      )
    `);
    expect(stderr).toContain("SECURITY ISSUE FOUND");
    expect(stderr).toContain(
      "SKILL.md L12-18: risk 8/10 — Requests exfiltration of SSH key",
    );
  });

  test("formats combined warnings", async () => {
    const { stderr } = await run(`
      agentSecurityBlock(
        [{ file: "SKILL.md", line: 5, category: "Tag injection", raw: "test" }],
        [{ file: "SKILL.md", lineRange: [10, 15], chunkIndex: 1, score: 9, reason: "Dangerous", raw: "test" }]
      )
    `);
    expect(stderr).toContain("SKILL.md L5: Tag injection");
    expect(stderr).toContain("SKILL.md L10-15: risk 9/10 — Dangerous");
  });
});
