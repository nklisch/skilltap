import { afterEach, beforeEach, describe, expect, setDefaultTimeout, test } from "bun:test";
setDefaultTimeout(45_000);
import { join } from "node:path";
import { mkdir } from "node:fs/promises";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runVerify(
  args: string[],
  cwd: string = CLI_DIR,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", `${CLI_DIR}/src/index.ts`, "verify", ...args],
    {
      cwd,
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

describe("verify — bare name resolution", () => {
  test("resolves bare name from .agents/skills/<name>", async () => {
    const skillDir = join(tmpDir, ".agents", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify(["my-skill"], tmpDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("SKILL.md found");
  });

  test("bare name falls back gracefully when not in .agents/skills", async () => {
    const skillDir = join(tmpDir, "other-skill");
    await mkdir(skillDir, { recursive: true });
    // my-skill not in .agents/skills — should fail as usual
    const { exitCode, stderr } = await runVerify(["my-skill"], tmpDir);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("No SKILL.md found");
  });
});

describe("verify --all", () => {
  test("verifies all skills and exits 0 when all valid", async () => {
    for (const name of ["skill-a", "skill-b"]) {
      const skillDir = join(tmpDir, ".agents", "skills", name);
      await mkdir(skillDir, { recursive: true });
      const skillMd = `---\nname: ${name}\ndescription: A test skill\nlicense: MIT\n---\n\n## Instructions\n\nDo stuff.\n`;
      await Bun.write(join(skillDir, "SKILL.md"), skillMd);
    }

    const { exitCode, stdout } = await runVerify(["--all"], tmpDir);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("skill-a");
    expect(stdout).toContain("skill-b");
  });

  test("exits 1 when any skill fails", async () => {
    const goodDir = join(tmpDir, ".agents", "skills", "good-skill");
    await mkdir(goodDir, { recursive: true });
    await Bun.write(join(goodDir, "SKILL.md"), `---\nname: good-skill\ndescription: Good\nlicense: MIT\n---\n\nDo stuff.\n`);

    const badDir = join(tmpDir, ".agents", "skills", "bad-skill");
    await mkdir(badDir, { recursive: true });
    await Bun.write(join(badDir, "SKILL.md"), `---\nname: bad-skill\n---\n\nMissing description.\n`);

    const { exitCode, stdout } = await runVerify(["--all"], tmpDir);
    expect(exitCode).toBe(1);
    expect(stdout).toContain("good-skill");
    expect(stdout).toContain("bad-skill");
  });

  test("exits 1 with error when no skills found", async () => {
    const { exitCode, stderr } = await runVerify(["--all"], tmpDir);
    expect(exitCode).toBe(1);
    expect(stderr).toContain("No skills found");
  });

  test("--all --json outputs JSON array", async () => {
    const skillDir = join(tmpDir, ".agents", "skills", "my-skill");
    await mkdir(skillDir, { recursive: true });
    await Bun.write(join(skillDir, "SKILL.md"), VALID_SKILL_MD);

    const { exitCode, stdout } = await runVerify(["--all", "--json"], tmpDir);
    expect(exitCode).toBe(0);
    const parsed = JSON.parse(stdout);
    expect(Array.isArray(parsed)).toBe(true);
    expect(parsed[0].name).toBe("my-skill");
    expect(parsed[0].valid).toBe(true);
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
