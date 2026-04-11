import { describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { parseAgentDefinitions } from "./agents";

describe("parseAgentDefinitions", () => {
  test("discovers all .md files in directory", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(join(agentsDir, "reviewer.md"), "---\nname: reviewer\n---\nReview content");
      await Bun.write(join(agentsDir, "builder.md"), "---\nname: builder\n---\nBuild content");

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(2);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("extracts name from frontmatter", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(join(agentsDir, "code-review.md"), "---\nname: code-reviewer\nmodel: sonnet\n---\nReview code");

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]?.name).toBe("code-reviewer");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("falls back to filename when no frontmatter name", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(join(agentsDir, "my-agent.md"), "# My Agent\nNo frontmatter here.");

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]?.name).toBe("my-agent");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("preserves full frontmatter dict", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(
        join(agentsDir, "agent.md"),
        "---\nname: agent\nmodel: sonnet\ntools: bash,editor\ncolor: blue\n---\nContent",
      );

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      const fm = result.value[0]?.frontmatter;
      expect(fm?.model).toBe("sonnet");
      expect(fm?.color).toBe("blue");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("returns ok([]) for non-existent directory", async () => {
    const result = await parseAgentDefinitions("/tmp/does-not-exist-skilltap-agents", "/tmp/does-not-exist-skilltap");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });

  test("returns ok([]) for empty directory", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toEqual([]);
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("ignores non-.md files", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(join(agentsDir, "README.txt"), "Not a markdown file");
      await Bun.write(join(agentsDir, "config.json"), '{"key":"value"}');
      await Bun.write(join(agentsDir, "agent.md"), "---\nname: agent\n---\nContent");

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
      expect(result.value[0]?.name).toBe("agent");
    } finally {
      await removeTmpDir(dir);
    }
  });

  test("path is relative to plugin root", async () => {
    const dir = await makeTmpDir();
    try {
      const agentsDir = join(dir, "agents");
      await mkdir(agentsDir, { recursive: true });
      await Bun.write(join(agentsDir, "reviewer.md"), "---\nname: reviewer\n---\nContent");

      const result = await parseAgentDefinitions(agentsDir, dir);
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]?.path).toBe("agents/reviewer.md");
    } finally {
      await removeTmpDir(dir);
    }
  });
});
