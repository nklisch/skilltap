import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, test } from "bun:test";
import type { Server } from "bun";
import { addTap, loadTaps, removeTap, searchTaps, updateTap } from "../../taps";

// Mock registry data
const REGISTRY_SKILLS = [
  {
    name: "code-review",
    description: "Thorough code review assistant",
    version: "2.0.0",
    tags: ["review", "quality"],
    source: { type: "git", url: "https://github.com/owner/code-review" },
    trust: { verified: true, verifiedBy: "registry-owner" },
  },
  {
    name: "commit-helper",
    description: "Generates conventional commit messages",
    version: "1.2.0",
    tags: ["git", "productivity"],
    source: { type: "github", repo: "owner/commit-helper" },
  },
  {
    name: "npm-skill",
    description: "A skill from npm",
    version: "3.0.0",
    tags: ["npm"],
    source: { type: "npm", package: "@scope/npm-skill", version: "3.0.0" },
  },
  {
    name: "url-skill",
    description: "A skill with direct tarball URL",
    version: "1.0.0",
    tags: ["url"],
    source: { type: "url", url: "https://registry.example.com/skills/url-skill.tar.gz" },
  },
];

let server: Server;
let baseUrl: string;
let configDir: string;
let origXdg: string | undefined;

beforeAll(() => {
  server = Bun.serve({
    port: 0,
    fetch(req) {
      const url = new URL(req.url);

      if (url.pathname === "/skills") {
        const q = url.searchParams.get("q")?.toLowerCase();
        const skills = q
          ? REGISTRY_SKILLS.filter(
              (s) =>
                s.name.includes(q) ||
                s.description.toLowerCase().includes(q) ||
                s.tags.some((t) => t.includes(q)),
            )
          : REGISTRY_SKILLS;
        return new Response(
          JSON.stringify({ skills, total: skills.length }),
          { headers: { "Content-Type": "application/json" } },
        );
      }

      for (const skill of REGISTRY_SKILLS) {
        if (url.pathname === `/skills/${skill.name}`) {
          return new Response(JSON.stringify({ ...skill, versions: [] }), {
            headers: { "Content-Type": "application/json" },
          });
        }
      }

      return new Response("Not Found", { status: 404 });
    },
  });
  baseUrl = `http://localhost:${server.port}`;
});

afterAll(() => {
  server.stop(true);
});

beforeEach(async () => {
  configDir = await mkdtemp(join(tmpdir(), "skilltap-http-test-"));
  origXdg = process.env.XDG_CONFIG_HOME;
  process.env.XDG_CONFIG_HOME = configDir;
});

afterEach(async () => {
  if (origXdg === undefined) {
    delete process.env.XDG_CONFIG_HOME;
  } else {
    process.env.XDG_CONFIG_HOME = origXdg;
  }
  await rm(configDir, { recursive: true, force: true });
});

describe("addTap with HTTP registry", () => {
  test("auto-detects HTTP registry", async () => {
    const result = await addTap("test-registry", baseUrl);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.type).toBe("http");
    expect(result.value.skillCount).toBe(REGISTRY_SKILLS.length);
  });

  test("accepts --type http override", async () => {
    const result = await addTap("test-registry", baseUrl, "http");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.type).toBe("http");
  });

  test("returns error if tap name already exists", async () => {
    await addTap("test-registry", baseUrl);
    const result = await addTap("test-registry", baseUrl);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("already exists");
  });
});

describe("loadTaps with HTTP registry", () => {
  test("loads skills from HTTP registry", async () => {
    await addTap("test-registry", baseUrl);

    const result = await loadTaps();
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const registryEntries = result.value.filter((e) => e.tapName === "test-registry");
    expect(registryEntries).toHaveLength(REGISTRY_SKILLS.length);
  });

  test("maps git source to plain URL", async () => {
    await addTap("test-registry", baseUrl);
    const result = await loadTaps();
    if (!result.ok) return;

    const codeReview = result.value.find((e) => e.skill.name === "code-review");
    expect(codeReview).toBeDefined();
    expect(codeReview?.skill.repo).toBe("https://github.com/owner/code-review");
  });

  test("maps github source to bare owner/repo", async () => {
    await addTap("test-registry", baseUrl);
    const result = await loadTaps();
    if (!result.ok) return;

    const commitHelper = result.value.find((e) => e.skill.name === "commit-helper");
    expect(commitHelper).toBeDefined();
    expect(commitHelper?.skill.repo).toBe("owner/commit-helper");
  });

  test("maps npm source with npm: prefix", async () => {
    await addTap("test-registry", baseUrl);
    const result = await loadTaps();
    if (!result.ok) return;

    const npmSkill = result.value.find((e) => e.skill.name === "npm-skill");
    expect(npmSkill).toBeDefined();
    expect(npmSkill?.skill.repo).toBe("npm:@scope/npm-skill");
  });

  test("maps url source with url: prefix", async () => {
    await addTap("test-registry", baseUrl);
    const result = await loadTaps();
    if (!result.ok) return;

    const urlSkill = result.value.find((e) => e.skill.name === "url-skill");
    expect(urlSkill).toBeDefined();
    expect(urlSkill?.skill.repo).toBe(
      "url:https://registry.example.com/skills/url-skill.tar.gz",
    );
  });

  test("includes trust info from registry", async () => {
    await addTap("test-registry", baseUrl);
    const result = await loadTaps();
    if (!result.ok) return;

    const codeReview = result.value.find((e) => e.skill.name === "code-review");
    expect(codeReview?.skill.trust?.verified).toBe(true);
    expect(codeReview?.skill.trust?.verifiedBy).toBe("registry-owner");
  });

  test("gracefully skips unreachable HTTP registries", async () => {
    // Add a reachable HTTP tap
    await addTap("good-registry", baseUrl);
    // Manually add an unreachable HTTP tap to config
    const { loadConfig, saveConfig } = await import("../../config");
    const configResult = await loadConfig();
    if (!configResult.ok) return;
    configResult.value.taps.push({
      name: "bad-registry",
      url: "http://localhost:1",
      type: "http",
    });
    await saveConfig(configResult.value);

    const result = await loadTaps();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    // Only entries from the good registry should appear
    const badEntries = result.value.filter((e) => e.tapName === "bad-registry");
    expect(badEntries).toHaveLength(0);
    const goodEntries = result.value.filter((e) => e.tapName === "good-registry");
    expect(goodEntries.length).toBeGreaterThan(0);
  });
});

describe("searchTaps with HTTP registry", () => {
  test("finds skills by name", async () => {
    await addTap("test-registry", baseUrl);
    const tapsResult = await loadTaps();
    if (!tapsResult.ok) return;

    const results = searchTaps(tapsResult.value, "review");
    expect(results.some((e) => e.skill.name === "code-review")).toBe(true);
  });

  test("finds skills by tag", async () => {
    await addTap("test-registry", baseUrl);
    const tapsResult = await loadTaps();
    if (!tapsResult.ok) return;

    const results = searchTaps(tapsResult.value, "productivity");
    expect(results.some((e) => e.skill.name === "commit-helper")).toBe(true);
  });
});

describe("updateTap with HTTP registry", () => {
  test("returns HTTP tap name in http array (no-op)", async () => {
    await addTap("test-registry", baseUrl);

    const result = await updateTap("test-registry");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.http).toContain("test-registry");
    expect(result.value.updated["test-registry"]).toBeUndefined();
  });

  test("separates git and HTTP taps in result", async () => {
    await addTap("http-tap", baseUrl);

    const result = await updateTap();
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.http).toContain("http-tap");
    expect(result.value.http).toHaveLength(1);
    expect(Object.keys(result.value.updated)).toHaveLength(0);
  });
});

describe("removeTap with HTTP registry", () => {
  test("removes HTTP tap from config without directory cleanup", async () => {
    await addTap("test-registry", baseUrl);

    const removeResult = await removeTap("test-registry");
    expect(removeResult.ok).toBe(true);

    const tapsResult = await loadTaps();
    if (!tapsResult.ok) return;
    expect(tapsResult.value.filter((e) => e.tapName === "test-registry")).toHaveLength(0);
  });
});
