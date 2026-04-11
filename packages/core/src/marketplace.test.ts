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

  test("maps marketplace name to tap name", () => {
    const tap = adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.name).toBe("test-marketplace");
  });

  test("maps metadata.description to tap description", () => {
    const tap = adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.description).toBe("A test marketplace");
  });

  test("tap description is undefined when metadata is absent", () => {
    const marketplace: Marketplace = { ...baseMarketplace, metadata: undefined };
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.description).toBeUndefined();
  });

  test("maps plugins to TapSkills with correct repo", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.name).toBe("gh-plugin");
    expect(tap.skills[0]?.repo).toBe("owner/repo");
    expect(tap.skills[0]?.description).toBe("A GitHub plugin");
    expect(tap.skills[0]?.plugin).toBe(true);
  });

  test("defaults description when missing", () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "no-desc-plugin",
          source: "./plugins/no-desc",
        },
      ],
    };
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.description).toBe(
      "Plugin from test-marketplace marketplace",
    );
  });

  test("uses plugin tags when provided", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual(["git", "productivity"]);
  });

  test("includes category in tags when tags absent", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual(["productivity"]);
  });

  test("tags is empty array when neither tags nor category present", () => {
    const marketplace: Marketplace = {
      ...baseMarketplace,
      plugins: [
        {
          name: "no-tags-plugin",
          source: "./plugins/no-tags",
        },
      ],
    };
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills[0]?.tags).toEqual([]);
  });

  test("skips plugins with null repo (unknown source type)", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.name).toBe("known-plugin");
  });

  test("deduplicates plugins by name (first wins)", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(1);
    expect(tap.skills[0]?.repo).toBe("owner/first");
    expect(tap.skills[0]?.description).toBe("First occurrence");
  });

  test("handles empty plugins array", () => {
    const tap = adaptMarketplaceToTap(baseMarketplace, TAP_URL);
    expect(tap.skills).toEqual([]);
  });

  test("multiple plugins of different sources all included", () => {
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
    const tap = adaptMarketplaceToTap(marketplace, TAP_URL);
    expect(tap.skills).toHaveLength(3);
    expect(tap.skills[0]?.repo).toBe(TAP_URL);
    expect(tap.skills[1]?.repo).toBe("o/r");
    expect(tap.skills[2]?.repo).toBe("npm:@org/pkg");
  });
});
