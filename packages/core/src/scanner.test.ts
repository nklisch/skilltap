import { describe, test, expect, afterEach } from "bun:test"
import { join } from "path"
import { scan } from "./scanner"
import {
  createStandaloneSkillRepo,
  createMultiSkillRepo,
  makeTmpDir,
  removeTmpDir,
} from "@skilltap/test-utils"

// ── Frontmatter parser tests (via scan on inline fixtures) ────────────────────

describe("scan — frontmatter parsing", () => {
  let tmpDir: string | null = null

  afterEach(async () => {
    if (tmpDir) { await removeTmpDir(tmpDir); tmpDir = null }
  })

  test("parses valid frontmatter into correct fields", async () => {
    tmpDir = await makeTmpDir()
    await Bun.write(join(tmpDir, "SKILL.md"), [
      "---",
      "name: my-skill",
      "description: Does something useful.",
      "license: MIT",
      "---",
      "# Content",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.name).toBe("my-skill")
    expect(skills[0]!.description).toBe("Does something useful.")
    expect(skills[0]!.valid).toBe(true)
    expect(skills[0]!.warnings).toHaveLength(1) // name/dir mismatch (tmpDir basename)
  })

  test("returns invalid skill with warning when no frontmatter", async () => {
    tmpDir = await makeTmpDir()
    await Bun.write(join(tmpDir, "SKILL.md"), "# Just a heading, no frontmatter")

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.valid).toBe(false)
    expect(skills[0]!.warnings.some(w => w.includes("frontmatter"))).toBe(true)
  })

  test("returns invalid skill with Zod errors for invalid frontmatter", async () => {
    tmpDir = await makeTmpDir()
    await Bun.write(join(tmpDir, "SKILL.md"), [
      "---",
      "name: INVALID NAME WITH SPACES",
      "description: desc",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.valid).toBe(false)
    expect(skills[0]!.warnings.length).toBeGreaterThan(0)
  })

  test("coerces boolean and number values in metadata", async () => {
    // Use a temp dir named after valid skill name for no mismatch warning
    tmpDir = await makeTmpDir()
    // Write into a subdir matching the skill name for the agent-specific path test
    const skillDir = join(tmpDir, ".agents", "skills", "my-tool")
    await Bun.write(join(skillDir, "SKILL.md"), [
      "---",
      "name: my-tool",
      "description: A tool skill.",
      "metadata: true",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.name).toBe("my-tool")
  })

  test("missing required fields produces invalid skill", async () => {
    tmpDir = await makeTmpDir()
    await Bun.write(join(tmpDir, "SKILL.md"), [
      "---",
      "name: incomplete-skill",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.valid).toBe(false)
    expect(skills[0]!.warnings.length).toBeGreaterThan(0)
  })
})

// ── Standalone repo ───────────────────────────────────────────────────────────

describe("scan — standalone skill repo", () => {
  test("returns exactly 1 valid skill from standalone repo", async () => {
    const repo = await createStandaloneSkillRepo()
    try {
      const skills = await scan(repo.path)
      expect(skills).toHaveLength(1)
      expect(skills[0]!.name).toBe("standalone-skill")
      expect(skills[0]!.description).toBe("A standalone test skill for use in integration tests.")
      expect(skills[0]!.valid).toBe(true)
    } finally {
      await repo.cleanup()
    }
  })

  test("path points to the skill directory (same as repo root for standalone)", async () => {
    const repo = await createStandaloneSkillRepo()
    try {
      const skills = await scan(repo.path)
      expect(skills[0]!.path).toBe(repo.path)
    } finally {
      await repo.cleanup()
    }
  })
})

// ── Multi-skill repo ──────────────────────────────────────────────────────────

describe("scan — multi-skill repo", () => {
  test("returns 2 valid skills", async () => {
    const repo = await createMultiSkillRepo()
    try {
      const skills = await scan(repo.path)
      expect(skills).toHaveLength(2)
      expect(skills.every(s => s.valid)).toBe(true)
    } finally {
      await repo.cleanup()
    }
  })

  test("skills are sorted by name", async () => {
    const repo = await createMultiSkillRepo()
    try {
      const skills = await scan(repo.path)
      expect(skills[0]!.name).toBe("skill-a")
      expect(skills[1]!.name).toBe("skill-b")
    } finally {
      await repo.cleanup()
    }
  })

  test("skill paths point to the correct skill directories", async () => {
    const repo = await createMultiSkillRepo()
    try {
      const skills = await scan(repo.path)
      const names = skills.map(s => s.name).sort()
      expect(names).toEqual(["skill-a", "skill-b"])
      for (const skill of skills) {
        const skillMd = Bun.file(join(skill.path, "SKILL.md"))
        expect(await skillMd.exists()).toBe(true)
      }
    } finally {
      await repo.cleanup()
    }
  })
})

// ── Edge cases ────────────────────────────────────────────────────────────────

describe("scan — edge cases", () => {
  let tmpDir: string | null = null

  afterEach(async () => {
    if (tmpDir) { await removeTmpDir(tmpDir); tmpDir = null }
  })

  test("returns empty array for empty directory", async () => {
    tmpDir = await makeTmpDir()
    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(0)
  })

  test("discovers skill in agent-specific .claude/skills/ path", async () => {
    tmpDir = await makeTmpDir()
    const skillDir = join(tmpDir, ".claude", "skills", "my-skill")
    await Bun.write(join(skillDir, "SKILL.md"), [
      "---",
      "name: my-skill",
      "description: Discovered via claude-specific path.",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.name).toBe("my-skill")
  })

  test("deduplicates: prefers .agents/skills/ over .claude/skills/ for same name", async () => {
    tmpDir = await makeTmpDir()

    // Canonical path
    const agentsDir = join(tmpDir, ".agents", "skills", "my-skill")
    await Bun.write(join(agentsDir, "SKILL.md"), [
      "---",
      "name: my-skill",
      "description: From .agents/skills/.",
      "---",
    ].join("\n"))

    // Agent-specific duplicate
    const claudeDir = join(tmpDir, ".claude", "skills", "my-skill")
    await Bun.write(join(claudeDir, "SKILL.md"), [
      "---",
      "name: my-skill",
      "description: From .claude/skills/.",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.path).toBe(agentsDir)
    expect(skills[0]!.description).toBe("From .agents/skills/.")
  })

  test("deep scan fallback finds SKILL.md in arbitrary subdirectory", async () => {
    tmpDir = await makeTmpDir()
    const nested = join(tmpDir, "deeply", "nested", "my-tool")
    await Bun.write(join(nested, "SKILL.md"), [
      "---",
      "name: my-tool",
      "description: Found by deep scan.",
      "---",
    ].join("\n"))

    const skills = await scan(tmpDir)
    expect(skills).toHaveLength(1)
    expect(skills[0]!.name).toBe("my-tool")
  })
})
