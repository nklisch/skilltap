import { describe, expect, test } from "bun:test";
import { MarketplaceSchema } from "./marketplace";

const VALID_RELATIVE_PLUGIN = {
  name: "my-plugin",
  source: "./plugins/my-plugin",
  description: "A relative path plugin",
};

const VALID_GITHUB_PLUGIN = {
  name: "github-plugin",
  source: { source: "github", repo: "owner/repo" },
  description: "A GitHub plugin",
};

const VALID_MARKETPLACE = {
  name: "test-marketplace",
  owner: { name: "Test Owner", email: "test@example.com" },
  metadata: { description: "A test marketplace", version: "1.0.0" },
  plugins: [VALID_RELATIVE_PLUGIN],
};

describe("MarketplaceSchema", () => {
  test("parses valid marketplace.json with relative path source", () => {
    const result = MarketplaceSchema.safeParse(VALID_MARKETPLACE);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.name).toBe("test-marketplace");
      expect(result.data.plugins[0]?.source).toBe("./plugins/my-plugin");
    }
  });

  test("parses valid marketplace.json with github source", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [VALID_GITHUB_PLUGIN],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      const source = result.data.plugins[0]?.source;
      expect(typeof source).toBe("object");
      if (typeof source === "object" && source !== null && "source" in source) {
        expect(source.source).toBe("github");
        expect((source as { repo: string }).repo).toBe("owner/repo");
      }
    }
  });

  test("parses valid marketplace.json with url source", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [
        {
          name: "url-plugin",
          source: { source: "url", url: "https://example.com/plugin.git" },
        },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      const source = result.data.plugins[0]?.source;
      expect(typeof source).toBe("object");
      if (typeof source === "object" && source !== null && "source" in source) {
        expect(source.source).toBe("url");
      }
    }
  });

  test("parses valid marketplace.json with git-subdir source", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [
        {
          name: "subdir-plugin",
          source: {
            source: "git-subdir",
            url: "https://example.com/mono.git",
            path: "packages/plugin",
          },
        },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      const source = result.data.plugins[0]?.source;
      expect(typeof source).toBe("object");
      if (typeof source === "object" && source !== null && "source" in source) {
        expect(source.source).toBe("git-subdir");
      }
    }
  });

  test("parses valid marketplace.json with npm source", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [
        {
          name: "npm-plugin",
          source: { source: "npm", package: "@org/my-plugin" },
        },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      const source = result.data.plugins[0]?.source;
      expect(typeof source).toBe("object");
      if (typeof source === "object" && source !== null && "source" in source) {
        expect(source.source).toBe("npm");
        expect((source as { package: string }).package).toBe("@org/my-plugin");
      }
    }
  });

  test("rejects missing name", () => {
    const { name: _, ...withoutName } = VALID_MARKETPLACE;
    expect(MarketplaceSchema.safeParse(withoutName).success).toBe(false);
  });

  test("rejects missing owner", () => {
    const { owner: _, ...withoutOwner } = VALID_MARKETPLACE;
    expect(MarketplaceSchema.safeParse(withoutOwner).success).toBe(false);
  });

  test("rejects missing plugins array", () => {
    const { plugins: _, ...withoutPlugins } = VALID_MARKETPLACE;
    expect(MarketplaceSchema.safeParse(withoutPlugins).success).toBe(false);
  });

  test("strips unknown fields (hooks, mcpServers, etc.)", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      strict: true,
      hooks: { preInstall: "echo hi" },
      mcpServers: ["some-server"],
      agents: { default: "claude" },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect("strict" in result.data).toBe(false);
      expect("hooks" in result.data).toBe(false);
      expect("mcpServers" in result.data).toBe(false);
    }
  });

  test("handles empty plugins array", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.plugins).toEqual([]);
    }
  });

  test("handles optional metadata", () => {
    const { metadata: _, ...withoutMetadata } = VALID_MARKETPLACE;
    const result = MarketplaceSchema.safeParse(withoutMetadata);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.metadata).toBeUndefined();
    }
  });

  test("parses plugin with optional fields (category, tags, version)", () => {
    const result = MarketplaceSchema.safeParse({
      ...VALID_MARKETPLACE,
      plugins: [
        {
          name: "full-plugin",
          source: "./plugins/full",
          description: "A full plugin",
          version: "2.0.0",
          category: "productivity",
          tags: ["git", "commit"],
        },
      ],
    });
    expect(result.success).toBe(true);
    if (result.success) {
      const plugin = result.data.plugins[0];
      expect(plugin?.category).toBe("productivity");
      expect(plugin?.tags).toEqual(["git", "commit"]);
      expect(plugin?.version).toBe("2.0.0");
    }
  });
});
