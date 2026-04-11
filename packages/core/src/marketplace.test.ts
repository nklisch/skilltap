import { describe, expect, test } from "bun:test";
import { adaptMarketplaceToTap, marketplaceSourceToRepo } from "./marketplace";
import type { Marketplace, MarketplacePluginSource } from "./schemas/marketplace";

const TAP_URL = "https://github.com/owner/marketplace.git";

describe("marketplaceSourceToRepo", () => {
  test("relative path string returns tapUrl", () => {
    const result = marketplaceSourceToRepo("./plugins/my-plugin", TAP_URL);
    expect(result).toBe(TAP_URL);
  });

  test("plain string (no leading ./) also returns tapUrl", () => {
    const result = marketplaceSourceToRepo("plugins/my-plugin", TAP_URL);
    expect(result).toBe(TAP_URL);
  });

  test("github source returns owner/repo", () => {
    const source: MarketplacePluginSource = {
      source: "github",
      repo: "owner/repo",
    };
    const result = marketplaceSourceToRepo(source, TAP_URL);
    expect(result).toBe("owner/repo");
  });

  test("url source returns the URL", () => {
    const source: MarketplacePluginSource = {
      source: "url",
      url: "https://example.com/plugin.git",
    };
    const result = marketplaceSourceToRepo(source, TAP_URL);
    expect(result).toBe("https://example.com/plugin.git");
  });

  test("git-subdir source returns the URL (path not preserved)", () => {
    const source: MarketplacePluginSource = {
      source: "git-subdir",
      url: "https://example.com/mono.git",
      path: "packages/plugin",
    };
    const result = marketplaceSourceToRepo(source, TAP_URL);
    expect(result).toBe("https://example.com/mono.git");
  });

  test("npm source returns npm:package", () => {
    const source: MarketplacePluginSource = {
      source: "npm",
      package: "@org/my-plugin",
    };
    const result = marketplaceSourceToRepo(source, TAP_URL);
    expect(result).toBe("npm:@org/my-plugin");
  });

  test("npm source without org scope", () => {
    const source: MarketplacePluginSource = {
      source: "npm",
      package: "my-plugin",
    };
    const result = marketplaceSourceToRepo(source, TAP_URL);
    expect(result).toBe("npm:my-plugin");
  });
});

describe("adaptMarketplaceToTap", () => {
  const baseMarketplace: Marketplace = {
    name: "test-marketplace",
    owner: { name: "Test Owner" },
    metadata: { description: "A test marketplace" },
    plugins: [],
  };

  test("maps marketplace name to tap name", async () => {
    const tap = await adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.name).toBe("test-marketplace");
  });

  test("maps metadata.description to tap description", async () => {
    const tap = await adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.description).toBe("A test marketplace");
  });

  test("tap description is undefined when metadata is absent", async () => {
    const marketplace: Marketplace = { ...baseMarketplace, metadata: undefined };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.description).toBeUndefined();
  });

  test("maps plugins to TapSkills with correct repo", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "gh-plugin",
          source: { source: "github", repo: "owner/repo" },
          description: "A GitHub plugin",
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.name).toBe("gh-plugin");
    expect(tap.skills[0]?.repo).toBe("owner/repo");
    expect(tap.skills[0]?.description).toBe("A GitHub plugin");
    expect(tap.skills[0]?.plugin).toBe(true);
  });

  test("defaults description when missing", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "no-desc-plugin",
          source: "./plugins/no-desc",
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.description).toBe(
      "Plugin from test-marketplace marketplace",
    );
  });

  test("uses plugin tags when provided", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "tagged-plugin",
          source: "./plugins/tagged",
          tags: ["git", "productivity"],
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual(["git", "productivity"]);
  });

  test("includes category in tags when tags absent", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "category-plugin",
          source: "./plugins/category",
          category: "productivity",
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual(["productivity"]);
  });

  test("tags is empty array when neither tags nor category present", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "no-tags-plugin",
          source: "./plugins/no-tags",
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual([]);
  });

  test("skips plugins with null repo (unknown source type)", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "unknown-plugin",
          // Cast to simulate an unknown/unexpected source object that passes at runtime
          source: { source: "unknown-type" } as unknown as MarketplacePluginSource,
        },
        {
          name: "known-plugin",
          source: { source: "github", repo: "owner/known" },
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.name).toBe("known-plugin");
  });

  test("deduplicates plugins by name (first wins)", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "dup-plugin",
          source: { source: "github", repo: "owner/first" },
          description: "First occurrence",
        },
        {
          name: "dup-plugin",
          source: { source: "github", repo: "owner/second" },
          description: "Second occurrence",
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.repo).toBe("owner/first");
    expect(tap.skills[0]?.description).toBe("First occurrence");
  });

  test("handles empty plugins array", async () => {
    const tap = await adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.skills).toEqual([]);
  });

  test("multiple plugins of different sources all included", async () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        { name: "relative-plugin", source: "./plugins/rel" },
        { name: "github-plugin", source: { source: "github", repo: "o/r" } },
        {
          name: "npm-plugin",
          source: { source: "npm", package: "@org/pkg" },
        },
      ],
    };
    const tap = await adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(3);
    expect(tap.skills[0]?.repo).toBe(TAP_URL);
    expect(tap.skills[1]?.repo).toBe("o/r");
    expect(tap.skills[2]?.repo).toBe("npm:@org/pkg");
  });

  test("detects plugin.json in relative-path source and produces TapPlugin entry", async () => {
    // Create a temp dir simulating a marketplace tap with a plugin that has .claude-plugin/plugin.json
    const { mkdir } = await import("node:fs/promises");
    const { mkdtemp } = await import("node:fs/promises");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");

    const tapDir = await mkdtemp(join(tmpdir(), "skilltap-mkt-test-"));
    try {
      // Create a plugin directory with .claude-plugin/plugin.json
      const pluginDir = join(tapDir, "plugins", "my-plugin");
      await mkdir(join(pluginDir, ".claude-plugin"), { recursive: true });
      await Bun.write(join(pluginDir, ".claude-plugin", "plugin.json"), JSON.stringify({ name: "my-plugin" }));

      // Add a skill
      await mkdir(join(pluginDir, "skills", "helper"), { recursive: true });
      await Bun.write(
        join(pluginDir, "skills", "helper", "SKILL.md"),
        "---\nname: helper\ndescription: A helper\n---\n# Helper\n",
      );

      // Add an MCP server
      await Bun.write(join(pluginDir, ".mcp.json"), JSON.stringify({ db: { command: "npx", args: ["-y", "db-mcp"] } }));

      // Add an agent
      await mkdir(join(pluginDir, "agents"), { recursive: true });
      await Bun.write(join(pluginDir, "agents", "reviewer.md"), "---\nname: reviewer\nmodel: sonnet\n---\nReview code.");

      const marketplace: Marketplace = {
        ...baseMarketplace,
        plugins: [
          {
            name: "my-plugin",
            source: "./plugins/my-plugin",
            description: "A full plugin",
            tags: ["dev"],
          },
        ],
      };

      const tap = await adaptMarketplaceToTap(marketplace, TAP_URL, tapDir);

      // Should produce a TapPlugin entry, NOT a TapSkill
      expect(tap.skills).toHaveLength(0);
      expect(tap.plugins).toHaveLength(1);

      const plugin = tap.plugins[0]!;
      expect(plugin.name).toBe("my-plugin");
      expect(plugin.description).toBe("A full plugin");
      expect(plugin.tags).toEqual(["dev"]);

      // Skills paths should be prefixed with the source path
      expect(plugin.skills.length).toBeGreaterThanOrEqual(1);
      expect(plugin.skills[0]?.name).toBe("helper");
      expect(plugin.skills[0]?.path).toContain("plugins/my-plugin");

      // MCP servers should be inline
      expect(plugin.mcpServers).toBeDefined();
      expect(typeof plugin.mcpServers).toBe("object");

      // Agents should be present
      expect(plugin.agents.length).toBeGreaterThanOrEqual(1);
      expect(plugin.agents[0]?.name).toBe("reviewer");
    } finally {
      const { rm } = await import("node:fs/promises");
      await rm(tapDir, { recursive: true, force: true });
    }
  });

  test("falls back to TapSkill when relative-path source has no plugin.json", async () => {
    const { mkdtemp, mkdir, rm } = await import("node:fs/promises");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");

    const tapDir = await mkdtemp(join(tmpdir(), "skilltap-mkt-test-"));
    try {
      // Create a plugin directory WITHOUT .claude-plugin/plugin.json
      const pluginDir = join(tapDir, "plugins", "simple");
      await mkdir(pluginDir, { recursive: true });

      const marketplace: Marketplace = {
        ...baseMarketplace,
        plugins: [
          { name: "simple", source: "./plugins/simple", description: "Just a skill" },
        ],
      };

      const tap = await adaptMarketplaceToTap(marketplace, TAP_URL, tapDir);

      // Should fall back to TapSkill (no plugin.json found)
      expect(tap.skills).toHaveLength(1);
      expect(tap.skills[0]?.name).toBe("simple");
      expect(tap.skills[0]?.plugin).toBe(true);
      expect(tap.plugins).toHaveLength(0);
    } finally {
      await rm(tapDir, { recursive: true, force: true });
    }
  });

  test("non-relative sources skip plugin detection even with tapDir", async () => {
    const { mkdtemp, rm } = await import("node:fs/promises");
    const { tmpdir } = await import("node:os");
    const { join } = await import("node:path");

    const tapDir = await mkdtemp(join(tmpdir(), "skilltap-mkt-test-"));
    try {
      const marketplace: Marketplace = {
        ...baseMarketplace,
        plugins: [
          { name: "gh-plugin", source: { source: "github", repo: "owner/repo" } },
        ],
      };

      const tap = await adaptMarketplaceToTap(marketplace, TAP_URL, tapDir);

      // GitHub source → TapSkill, not scanned for plugin.json
      expect(tap.skills).toHaveLength(1);
      expect(tap.skills[0]?.name).toBe("gh-plugin");
      expect(tap.plugins).toHaveLength(0);
    } finally {
      await rm(tapDir, { recursive: true, force: true });
    }
  });
});
