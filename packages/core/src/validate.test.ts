import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { mkdir, rm } from "node:fs/promises";
import { validateSkill } from "./validate";

async function makeTmpDir(): Promise<string> {
  const dir = `/tmp/skilltap-validate-${crypto.randomUUID()}`;
  await mkdir(dir, { recursive: true });
  return dir;
}

async function cleanup(dir: string): Promise<void> {
  await rm(dir, { recursive: true, force: true });
}

async function writeFile(dir: string, rel: string, content: string): Promise<void> {
  const fullPath = join(dir, rel);
  await Bun.write(fullPath, content);
}

const VALID_SKILL_MD = `---
name: my-skill
description: A test skill for validation
license: MIT
---

## Instructions

Do stuff.
`;

describe("validateSkill", () => {
  test("returns error when no SKILL.md found", async () => {
    const dir = await makeTmpDir();
    try {
      const result = await validateSkill(dir);
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("No SKILL.md found");
    } finally {
      await cleanup(dir);
    }
  });

  test("valid skill returns valid=true with no errors", async () => {
    // Use a tmp dir named 'my-skill' so name matches
    const parentDir = await makeTmpDir();
    const dir = join(parentDir, "my-skill");
    await mkdir(dir, { recursive: true });
    try {
      await writeFile(dir, "SKILL.md", VALID_SKILL_MD);
      const result = await validateSkill(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.valid).toBe(true);
      const errors = result.value.issues.filter((i) => i.severity === "error");
      expect(errors).toHaveLength(0);
      expect(result.value.frontmatter?.name).toBe("my-skill");
      expect(result.value.frontmatter?.description).toBe("A test skill for validation");
    } finally {
      await cleanup(parentDir);
    }
  });

  test("missing frontmatter returns invalid with error", async () => {
    const parentDir = await makeTmpDir();
    const dir = join(parentDir, "my-skill");
    await mkdir(dir, { recursive: true });
    try {
      await writeFile(dir, "SKILL.md", "# Just some content\n\nNo frontmatter here.\n");
      const result = await validateSkill(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.valid).toBe(false);
      const errors = result.value.issues.filter((i) => i.severity === "error");
      expect(errors.length).toBeGreaterThan(0);
      expect(errors[0]?.message).toContain("No YAML frontmatter");
    } finally {
      await cleanup(parentDir);
    }
  });

  test("invalid frontmatter schema returns invalid with error", async () => {
    const parentDir = await makeTmpDir();
    const dir = join(parentDir, "my-skill");
    await mkdir(dir, { recursive: true });
    try {
      // Missing required 'description' field
      const badSkillMd = `---
name: my-skill
---

## Instructions
`;
      await writeFile(dir, "SKILL.md", badSkillMd);
      const result = await validateSkill(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.valid).toBe(false);
      const errors = result.value.issues.filter((i) => i.severity === "error");
      expect(errors.length).toBeGreaterThan(0);
    } finally {
      await cleanup(parentDir);
    }
  });

  test("name mismatch with directory name returns error", async () => {
    const parentDir = await makeTmpDir();
    const dir = join(parentDir, "wrong-dir-name");
    await mkdir(dir, { recursive: true });
    try {
      // name is 'my-skill' but dir is 'wrong-dir-name'
      await writeFile(dir, "SKILL.md", VALID_SKILL_MD);
      const result = await validateSkill(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.valid).toBe(false);
      const errors = result.value.issues.filter((i) => i.severity === "error");
      expect(errors.some((e) => e.message.includes("does not match directory name"))).toBe(true);
    } finally {
      await cleanup(parentDir);
    }
  });

  test("returns fileCount and totalBytes", async () => {
    const parentDir = await makeTmpDir();
    const dir = join(parentDir, "my-skill");
    await mkdir(dir, { recursive: true });
    try {
      await writeFile(dir, "SKILL.md", VALID_SKILL_MD);
      const result = await validateSkill(dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.fileCount).toBeGreaterThan(0);
      expect(result.value.totalBytes).toBeGreaterThan(0);
    } finally {
      await cleanup(parentDir);
    }
  });
});
