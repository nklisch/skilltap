import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";

const CLI_DIR = `${import.meta.dir}/../..`;

async function runCreate(
  args: string[],
  cwd: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn(
    ["bun", "run", "--bun", "src/index.ts", "create", ...args],
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

let tmpDir: string;

beforeEach(async () => {
  tmpDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(tmpDir);
});

describe("create — basic template", () => {
  test("creates SKILL.md and .gitignore in output dir", async () => {
    const outDir = join(tmpDir, "my-skill");
    const { exitCode, stdout } = await runCreate(
      ["my-skill", "--template", "basic", "--dir", outDir],
      tmpDir,
    );
    expect(exitCode).toBe(0);
    expect(stdout).toContain("SKILL.md");

    const skillMd = await Bun.file(join(outDir, "SKILL.md")).text();
    expect(skillMd).toContain("name: my-skill");

    const gitignore = await Bun.file(join(outDir, ".gitignore")).text();
    expect(gitignore).toContain("node_modules/");
  });

  test("exits 1 if output dir already exists", async () => {
    const outDir = join(tmpDir, "my-skill");
    await Bun.$`mkdir -p ${outDir}`.quiet();
    const { exitCode, stderr } = await runCreate(
      ["my-skill", "--template", "basic", "--dir", outDir],
      tmpDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("already exists");
  });

  test("exits 1 for invalid skill name", async () => {
    const outDir = join(tmpDir, "bad_name");
    const { exitCode, stderr } = await runCreate(
      ["bad_name", "--template", "basic", "--dir", outDir],
      tmpDir,
    );
    expect(exitCode).toBe(1);
    expect(stderr).toContain("lowercase alphanumeric");
  });
});

describe("create — npm template", () => {
  test("creates SKILL.md, package.json, .gitignore, and publish workflow", async () => {
    const outDir = join(tmpDir, "my-npm-skill");
    const { exitCode } = await runCreate(
      ["my-npm-skill", "--template", "npm", "--dir", outDir],
      tmpDir,
    );
    expect(exitCode).toBe(0);

    expect(await Bun.file(join(outDir, "SKILL.md")).exists()).toBe(true);
    expect(await Bun.file(join(outDir, "package.json")).exists()).toBe(true);
    expect(await Bun.file(join(outDir, ".gitignore")).exists()).toBe(true);
    expect(
      await Bun.file(join(outDir, ".github/workflows/publish.yml")).exists(),
    ).toBe(true);

    const pkg = JSON.parse(await Bun.file(join(outDir, "package.json")).text());
    expect(pkg.keywords).toContain("agent-skill");
    expect(pkg.name).toBe("my-npm-skill");
  });
});

describe("create — multi template", () => {
  test("creates .agents/skills/ structure", async () => {
    const outDir = join(tmpDir, "multi-skills");
    const { exitCode } = await runCreate(
      ["multi-skills", "--template", "multi", "--dir", outDir],
      tmpDir,
    );
    // multi template in non-interactive mode creates two default skills
    expect(exitCode).toBe(0);
    expect(
      await Bun.file(join(outDir, ".agents/skills/multi-skills-a/SKILL.md")).exists(),
    ).toBe(true);
    expect(
      await Bun.file(join(outDir, ".agents/skills/multi-skills-b/SKILL.md")).exists(),
    ).toBe(true);
  });
});

describe("create — next steps output", () => {
  test("prints next steps including skilltap verify", async () => {
    const outDir = join(tmpDir, "my-skill");
    const { stdout } = await runCreate(
      ["my-skill", "--template", "basic", "--dir", outDir],
      tmpDir,
    );
    expect(stdout).toContain("skilltap verify");
    expect(stdout).toContain("skilltap link");
  });
});
