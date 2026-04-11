import { describe, expect, test } from "bun:test";
import { TapPluginSchema, TapSchema, TapSkillSchema } from "./tap";

const VALID_SKILL = {
  name: "commit-helper",
  description: "Generates conventional commit messages",
  repo: "https://gitea.example.com/nathan/commit-helper",
  tags: ["git", "productivity"],
};

describe("TapSkillSchema", () => {
  test("accepts a fully populated skill", () => {
    const result = TapSkillSchema.safeParse(VALID_SKILL);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("commit-helper");
      expect(result.data.tags).toEqual(["git", "productivity"]);
    }
  });

  test("defaults tags to empty array when omitted", () => {
    const { tags: _, ...withoutTags } = VALID_SKILL;
    const result = TapSkillSchema.safeParse(withoutTags);
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.tags).toEqual([]);
  });

  test("rejects missing name", () => {
    const { name: _, ...withoutName } = VALID_SKILL;
    expect(TapSkillSchema.safeParse(withoutName).success).toBe(false);
  });

  test("rejects missing description", () => {
    const { description: _, ...withoutDesc } = VALID_SKILL;
    expect(TapSkillSchema.safeParse(withoutDesc).success).toBe(false);
  });

  test("rejects missing repo", () => {
    const { repo: _, ...withoutRepo } = VALID_SKILL;
    expect(TapSkillSchema.safeParse(withoutRepo).success).toBe(false);
  });
});

describe("TapSchema", () => {
  test("accepts full tap with description", () => {
    const result = TapSchema.safeParse({
      name: "nathan's skills",
      description: "My curated skill collection",
      skills: [VALID_SKILL],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("nathan's skills");
      expect(result.data.description).toBe("My curated skill collection");
      expect(result.data.skills).toHaveLength(1);
    }
  });

  test("accepts tap without description (optional)", () => {
    const result = TapSchema.safeParse({
      name: "my-tap",
      skills: [],
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.description).toBeUndefined();
  });

  test("accepts empty skills array", () => {
    const result = TapSchema.safeParse({ name: "empty-tap", skills: [] });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.skills).toEqual([]);
  });

  test("accepts multiple skills", () => {
    const result = TapSchema.safeParse({
      name: "multi-tap",
      skills: [VALID_SKILL, { ...VALID_SKILL, name: "other-skill", tags: [] }],
    });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.skills).toHaveLength(2);
  });

  test("rejects missing name", () => {
    expect(TapSchema.safeParse({ skills: [] }).success).toBe(false);
  });

  test("rejects missing skills array", () => {
    expect(TapSchema.safeParse({ name: "tap" }).success).toBe(false);
  });

  test("propagates skill validation errors", () => {
    expect(
      TapSchema.safeParse({
        name: "tap",
        skills: [{ ...VALID_SKILL, repo: 123 }],
      }).success,
    ).toBe(false);
  });

  test("defaults plugins to [] when omitted", () => {
    const result = TapSchema.safeParse({ name: "my-tap", skills: [] });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.plugins).toEqual([]);
  });

  test("accepts tap.json with plugins array", () => {
    const result = TapSchema.safeParse({
      name: "my-tap",
      skills: [],
      plugins: [
        {
          name: "dev-toolkit",
          description: "Dev tools",
          version: "1.0.0",
          skills: [{ name: "code-review", path: "plugins/code-review" }],
          mcpServers: { "test-db": { command: "npx", args: ["-y", "test-mcp"] } },
          agents: [{ name: "reviewer", path: "plugins/agents/reviewer.md" }],
          tags: ["dev"],
        },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.plugins).toHaveLength(1);
      expect(result.data.plugins[0]!.name).toBe("dev-toolkit");
    }
  });
});

describe("TapPluginSchema", () => {
  test("requires name", () => {
    expect(TapPluginSchema.safeParse({}).success).toBe(false);
  });

  test("defaults description to empty string", () => {
    const result = TapPluginSchema.safeParse({ name: "my-plugin" });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.description).toBe("");
  });

  test("defaults skills to []", () => {
    const result = TapPluginSchema.safeParse({ name: "my-plugin" });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.skills).toEqual([]);
  });

  test("defaults agents to []", () => {
    const result = TapPluginSchema.safeParse({ name: "my-plugin" });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.agents).toEqual([]);
  });

  test("defaults tags to []", () => {
    const result = TapPluginSchema.safeParse({ name: "my-plugin" });
    expect(result.success).toBe(true);
    if (result.success) expect(result.data.tags).toEqual([]);
  });

  test("accepts mcpServers as inline object", () => {
    const result = TapPluginSchema.safeParse({
      name: "my-plugin",
      mcpServers: { "my-db": { command: "npx", args: [] } },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(typeof result.data.mcpServers).toBe("object");
    }
  });

  test("accepts mcpServers as string path", () => {
    const result = TapPluginSchema.safeParse({
      name: "my-plugin",
      mcpServers: "plugins/my-plugin/.mcp.json",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.mcpServers).toBe("plugins/my-plugin/.mcp.json");
    }
  });

  test("accepts plugin with all fields", () => {
    const result = TapPluginSchema.safeParse({
      name: "full-plugin",
      description: "Full plugin",
      version: "2.0.0",
      skills: [{ name: "my-skill", path: "skills/my-skill", description: "A skill" }],
      mcpServers: { db: { command: "npx", args: [] } },
      agents: [{ name: "my-agent", path: "agents/my-agent.md" }],
      tags: ["a", "b"],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.version).toBe("2.0.0");
      expect(result.data.skills).toHaveLength(1);
      expect(result.data.agents).toHaveLength(1);
      expect(result.data.tags).toEqual(["a", "b"]);
    }
  });
});
