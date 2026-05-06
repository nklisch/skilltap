import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pluginV2ToManifest } from "./normalize";
import type { PluginManifestV2 } from "./schema";

let repoRoot: string;
beforeEach(async () => {
  repoRoot = await mkdtemp(join(tmpdir(), "skilltap-norm-"));
});
afterEach(async () => {
  await rm(repoRoot, { recursive: true, force: true });
});

describe("pluginV2ToManifest", () => {
  test("converts a minimal v2 manifest", async () => {
    const v2: PluginManifestV2 = {
      name: "minimal",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [],
      servers: [],
      agents: [],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toMatchObject({
      name: "minimal",
      version: "1.0.0",
      format: "skilltap",
      pluginRoot: repoRoot,
      components: [],
    });
  });

  test("preserves declared skills when SKILL.md is missing", async () => {
    const v2: PluginManifestV2 = {
      name: "test",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [{ name: "code-review", path: "./skills/code-review", description: "" }],
      servers: [],
      agents: [],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(1);
    expect(result.value.components[0]).toMatchObject({
      type: "skill",
      name: "code-review",
    });
  });

  test("scans skill dir and uses SKILL.md frontmatter", async () => {
    const skillsDir = join(repoRoot, "skills", "review");
    await mkdir(skillsDir, { recursive: true });
    await writeFile(
      join(skillsDir, "SKILL.md"),
      `---
name: review
description: A real skill
---

# Review

Body here.
`,
    );
    const v2: PluginManifestV2 = {
      name: "test",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [{ name: "ignored-name", path: "./skills/review", description: "" }],
      servers: [],
      agents: [],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(1);
    const skill = result.value.components[0] as { type: string; name: string; description: string };
    expect(skill.type).toBe("skill");
    expect(skill.name).toBe("review");
    expect(skill.description).toBe("A real skill");
  });

  test("converts stdio servers", async () => {
    const v2: PluginManifestV2 = {
      name: "test",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [],
      servers: [
        {
          type: "stdio",
          name: "db",
          command: "node",
          args: ["s.js"],
          env: { X: "1" },
        },
      ],
      agents: [],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(1);
    const mcp = result.value.components[0] as { type: string; server: { type: string; name: string } };
    expect(mcp.type).toBe("mcp");
    expect(mcp.server.type).toBe("stdio");
    expect(mcp.server.name).toBe("db");
  });

  test("converts http servers", async () => {
    const v2: PluginManifestV2 = {
      name: "test",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [],
      servers: [
        {
          type: "http",
          name: "search",
          url: "https://search.example.com/mcp",
          headers: { Authorization: "Bearer x" },
        },
      ],
      agents: [],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(1);
    const mcp = result.value.components[0] as { type: string; server: { type: string; url: string } };
    expect(mcp.server.type).toBe("http");
    expect(mcp.server.url).toBe("https://search.example.com/mcp");
  });

  test("reads agents from .md files with frontmatter", async () => {
    const agentsDir = join(repoRoot, "agents");
    await mkdir(agentsDir, { recursive: true });
    await writeFile(
      join(agentsDir, "reviewer.md"),
      `---
name: reviewer
description: Reviews code
model: sonnet
---

Body of agent prompt.
`,
    );

    const v2: PluginManifestV2 = {
      name: "test",
      version: "1.0.0",
      description: "",
      publish: true,
      skills: [],
      servers: [],
      agents: [{ name: "reviewer", path: "./agents/reviewer.md" }],
    };
    const result = await pluginV2ToManifest(v2, repoRoot);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.components).toHaveLength(1);
    const agent = result.value.components[0] as {
      type: string;
      name: string;
      frontmatter: Record<string, unknown>;
    };
    expect(agent.type).toBe("agent");
    expect(agent.name).toBe("reviewer");
    expect(agent.frontmatter.model).toBe("sonnet");
  });
});
