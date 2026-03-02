import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import type { Server } from "bun";
import { detectTapType, fetchSkillDetail, fetchSkillList } from "../client";

const SKILLS_LIST = {
  skills: [
    {
      name: "commit-helper",
      description: "Generates conventional commit messages",
      version: "1.2.0",
      author: "nathan",
      tags: ["git"],
      source: { type: "git", url: "https://github.com/owner/commit-helper" },
    },
  ],
  total: 1,
};

const SKILL_DETAIL = {
  name: "commit-helper",
  description: "Generates conventional commit messages",
  author: "nathan",
  license: "MIT",
  tags: ["git"],
  versions: [{ version: "1.2.0", publishedAt: "2026-01-01T00:00:00Z" }],
  source: { type: "git", url: "https://github.com/owner/commit-helper" },
};

let server: Server;
let baseUrl: string;
let lastAuthHeader: string | null = null;

beforeAll(() => {
  server = Bun.serve({
    port: 0,
    fetch(req) {
      const url = new URL(req.url);
      lastAuthHeader = req.headers.get("Authorization");

      if (url.pathname === "/skills") {
        // Simulate 401 for certain tokens
        if (lastAuthHeader === "Bearer bad-token") {
          return new Response(JSON.stringify({ error: "Unauthorized" }), { status: 401 });
        }
        if (lastAuthHeader === "Bearer rate-limited") {
          return new Response("Too Many Requests", { status: 429 });
        }
        return new Response(JSON.stringify(SKILLS_LIST), {
          headers: { "Content-Type": "application/json" },
        });
      }

      if (url.pathname === "/skills/commit-helper") {
        return new Response(JSON.stringify(SKILL_DETAIL), {
          headers: { "Content-Type": "application/json" },
        });
      }

      if (url.pathname === "/skills/nonexistent") {
        return new Response(JSON.stringify({ error: "Not found" }), { status: 404 });
      }

      return new Response("Not Found", { status: 404 });
    },
  });
  baseUrl = `http://localhost:${server.port}`;
});

afterAll(() => {
  server.stop(true);
});

describe("fetchSkillList", () => {
  test("fetches skills from registry", async () => {
    const result = await fetchSkillList(baseUrl, "test", {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skills).toHaveLength(1);
    expect(result.value.skills[0]?.name).toBe("commit-helper");
    expect(result.value.total).toBe(1);
  });

  test("sends Bearer token when auth_token provided", async () => {
    await fetchSkillList(baseUrl, "test", { token: "my-secret-token" });
    expect(lastAuthHeader).toBe("Bearer my-secret-token");
  });

  test("reads token from env var when auth_env provided", async () => {
    process.env.TEST_REGISTRY_TOKEN = "env-token-value";
    await fetchSkillList(baseUrl, "test", { envVar: "TEST_REGISTRY_TOKEN" });
    expect(lastAuthHeader).toBe("Bearer env-token-value");
    delete process.env.TEST_REGISTRY_TOKEN;
  });

  test("sends no auth header when no auth configured", async () => {
    await fetchSkillList(baseUrl, "test", {});
    expect(lastAuthHeader).toBeNull();
  });

  test("returns auth error for 401", async () => {
    const result = await fetchSkillList(baseUrl, "test", { token: "bad-token" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Authentication required");
    expect(result.error.message).toContain("test");
  });

  test("returns network error for 429 (rate limited)", async () => {
    const result = await fetchSkillList(baseUrl, "test", { token: "rate-limited" });
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Rate limited");
  });

  test("returns error for unreachable registry", async () => {
    const result = await fetchSkillList("http://localhost:1", "test", {});
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("Could not reach registry");
  });
});

describe("fetchSkillDetail", () => {
  test("fetches skill detail", async () => {
    const result = await fetchSkillDetail(baseUrl, "test", "commit-helper", {});
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.name).toBe("commit-helper");
    expect(result.value.author).toBe("nathan");
    expect(result.value.versions).toHaveLength(1);
  });

  test("returns UserError for 404", async () => {
    const result = await fetchSkillDetail(baseUrl, "test", "nonexistent", {});
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.message).toContain("not found");
    expect(result.error.message).toContain("nonexistent");
  });
});

describe("detectTapType", () => {
  test("detects HTTP registry", async () => {
    const type = await detectTapType(baseUrl);
    expect(type).toBe("http");
  });

  test("returns git for non-registry URLs", async () => {
    const type = await detectTapType("http://localhost:1");
    expect(type).toBe("git");
  });
});
