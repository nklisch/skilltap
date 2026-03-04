import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(15_000);
import { join } from "node:path";
import { mkdir } from "node:fs/promises";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runVerify(
  args: string[],
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "verify", ...args],
    {
      cwd: CLI_DIR,
      stdout: "pipe",
      stderr: "pipe",
      stdin: "pipe",
      env: {
        ...process.env,
      },
    },
  );
  const exitCode = await proc.exited;
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  return { exitCode, stdout, stderr };
}

const VALID_SKILL_MD = `---
name: my-skill
description: A test skill for verification
license: MIT
---

## Instructions

Do stuff.
`;

let tmpDir: string;

beforeEach(async () => {
  tmpDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(tmpDir);
});

describe("verify — valid skill", () => {
  test("exits 0 and shows valid output", async () => {
    const skillDir = join(tmpDir, "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify([skillDir]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("SKILL.md found");
    expect(stdout).toContain("Frontmatter valid");
    expect(stdout).toContain("Security scan: clean");
  });

  test("prints tap.json snippet on valid skill", async () => {
    const skillDir = join(tmpDir, "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify([skillDir]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("tap.json");
    expect(stdout).toContain("my-skill");
  });
});

describe("verify — missing SKILL.md", () => {
  test("exits 1 with error message", async () => {
    const skillDir = join(tmpDir, "empty-skill");
    await mkdir(skillDir, { recursive: true });

    const { exitCode, stderr } = await runVerify([skillDir]);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("No SKILL.md found");
  });
});

describe("verify — invalid frontmatter", () => {
  test("exits 1 and shows error", async () => {
    const skillDir = join(tmpDir, "bad-skill");
    await mkdir(skillDir, { recursive: true });
    // Missing required description field
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---
name: bad-skill
---

## Instructions
`,
    );

    const { exitCode, stdout } = await runVerify([skillDir]);
    expect(exitCode).toBe(1);
    expect(stdout).toContain("Fix");
  });
});

describe("verify — name mismatch", () => {
  test("exits 1 when frontmatter name doesn't match directory", async () => {
    const skillDir = join(tmpDir, "wrong-name");
    await mkdir(skillDir, { recursive: true });
    // frontmatter name is 'my-skill' but dir is 'wrong-name'
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify([skillDir]);
    expect(exitCode).toBe(1);
    expect(stdout).toContain("does not match directory name");
  });
});

describe("verify --json", () => {
  test("outputs JSON on valid skill", async () => {
    const skillDir = join(tmpDir, "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify([skillDir, "--json"]);
    expect(exitCode).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(parsed.valid).toBe(true);
    expect(parsed.name).toBe("my-skill");
    expect(parsed.frontmatter.name).toBe("my-skill");
  });

  test("outputs JSON with valid=false on invalid skill", async () => {
    const skillDir = join(tmpDir, "my-skill");
    await mkdir(skillDir, { recursive: true });
    // Missing description
    await Bun.write(
      join(skillDir, "SKILL.md"),
      `---
name: my-skill
---

## Instructions
`,
    );

    const { exitCode, stdout } = await runVerify([skillDir, "--json"]);
    expect(exitCode).toBe(1);
    const parsed = JSON.parse(stdout);
    expect(parsed.valid).toBe(false);
    expect(parsed.issues.length).toBeGreaterThan(0);
  });
});

describe("verify + create roundtrip", () => {
  test("skill created by create command passes verify", async () => {
    const outDir = join(tmpDir, "round-trip-skill");
    // Use create to generate the skill
    const createProc = Bun.spawn(
      [
        "bun", "run", "--bun", "src/index.ts", "create",
        "round-trip-skill", "--template", "basic", "--dir", outDir,
      ],
      {
        cwd: `${import.meta.dir}/../..`,
        stdout: "pipe",
        stderr: "pipe",
        stdin: "pipe",
        env: { ...process.env },
      },
    );
    await createProc.exited;

    // Now verify it
    const { exitCode } = await runVerify([outDir]);
    expect(exitCode).toBe(0);
  });
});
