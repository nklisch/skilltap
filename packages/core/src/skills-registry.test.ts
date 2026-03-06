import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { ConfigSchema } from "./schemas/config";
import {
  createCustomRegistry,
  resolveRegistries,
  searchRegistries,
  searchSkillsRegistry,
  skillsShRegistry,
} from "./skills-registry";

// ---------------------------------------------------------------------------
// Fetch mock helpers
// ---------------------------------------------------------------------------

const originalFetch = globalThis.fetch;

function makeFetch(status: number, body: unknown): typeof fetch {
  return mock(() =>
    Promise.resolve({
      ok: status >= 200 && status < 300,
      json: () => Promise.resolve(body),
    } as Response),
  );
}

function makeFetchThrow(): typeof fetch {
  return mock(() => Promise.reject(new Error("network error")));
}

afterEach(() => {
  globalThis.fetch = originalFetch;
});

const SKILL_FIXTURE = {
  id: "owner/repo/my-skill",
  name: "my-skill",
  description: "Does things",
  source: "github:owner/repo",
  installs: 42,
};

const REGISTRY_RESPONSE = { skills: [SKILL_FIXTURE] };

// ---------------------------------------------------------------------------
// resolveRegistries — pure, no fetch needed
// ---------------------------------------------------------------------------

describe("resolveRegistries", () => {
  const base = ConfigSchema.parse({});

  test("default config returns skillsShRegistry", () => {
    const result = resolveRegistries(base);
    expect(result).toHaveLength(1);
    expect(result[0]?.name).toBe("skills.sh");
  });

  test("enabled: [] returns empty array", () => {
    const config = { ...base, registry: { ...base.registry, enabled: [] } };
    expect(resolveRegistries(config)).toHaveLength(0);
  });

  test("unknown name with no matching source is skipped", () => {
    const config = { ...base, registry: { ...base.registry, enabled: ["nonexistent"] } };
    expect(resolveRegistries(config)).toHaveLength(0);
  });

  test("custom source name returns registry with correct name", () => {
    const config = {
      ...base,
      registry: {
        ...base.registry,
        enabled: ["my-registry"],
        sources: [{ name: "my-registry", url: "https://example.com" }],
      },
    };
    const result = resolveRegistries(config);
    expect(result).toHaveLength(1);
    expect(result[0]?.name).toBe("my-registry");
  });

  test("multiple enabled registries returned in order", () => {
    const config = {
      ...base,
      registry: {
        ...base.registry,
        enabled: ["skills.sh", "second"],
        sources: [{ name: "second", url: "https://second.example.com" }],
      },
    };
    const result = resolveRegistries(config);
    expect(result).toHaveLength(2);
    expect(result[0]?.name).toBe("skills.sh");
    expect(result[1]?.name).toBe("second");
  });
});

// ---------------------------------------------------------------------------
// searchRegistries — mock registry objects, no fetch
// ---------------------------------------------------------------------------

describe("searchRegistries", () => {
  test("empty registries returns []", async () => {
    expect(await searchRegistries("test", [])).toEqual([]);
  });

  test("single registry — results tagged with registryName", async () => {
    const reg = {
      name: "test-reg",
      search: async () => [{ ...SKILL_FIXTURE }],
    };
    const results = await searchRegistries("test", [reg]);
    expect(results).toHaveLength(1);
    expect(results[0]?.registryName).toBe("test-reg");
    expect(results[0]?.name).toBe("my-skill");
  });

  test("two registries — results concatenated, each tagged", async () => {
    const reg1 = { name: "r1", search: async () => [{ ...SKILL_FIXTURE, id: "a" }] };
    const reg2 = { name: "r2", search: async () => [{ ...SKILL_FIXTURE, id: "b" }] };
    const results = await searchRegistries("test", [reg1, reg2]);
    expect(results).toHaveLength(2);
    expect(results.find((r) => r.registryName === "r1")?.id).toBe("a");
    expect(results.find((r) => r.registryName === "r2")?.id).toBe("b");
  });

  test("registry returning [] contributes nothing", async () => {
    const reg1 = { name: "r1", search: async () => [{ ...SKILL_FIXTURE }] };
    const reg2 = { name: "r2", search: async (): Promise<typeof SKILL_FIXTURE[]> => [] };
    const results = await searchRegistries("test", [reg1, reg2]);
    expect(results).toHaveLength(1);
    expect(results[0]?.registryName).toBe("r1");
  });

  test("respects custom limit", async () => {
    let capturedLimit = 0;
    const reg = {
      name: "r",
      search: async (_q: string, limit: number) => {
        capturedLimit = limit;
        return [];
      },
    };
    await searchRegistries("test", [reg], 5);
    expect(capturedLimit).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// createCustomRegistry — fetch mock
// ---------------------------------------------------------------------------

describe("createCustomRegistry", () => {
  test("returns registry with correct name", () => {
    const reg = createCustomRegistry("my-reg", "https://example.com");
    expect(reg.name).toBe("my-reg");
  });

  test("strips trailing slash from base URL", async () => {
    let capturedUrl = "";
    globalThis.fetch = mock((url: string | URL | Request) => {
      capturedUrl = url.toString();
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ skills: [] }) } as Response);
    });
    const reg = createCustomRegistry("r", "https://example.com/");
    await reg.search("test", 10);
    expect(capturedUrl).toContain("https://example.com/api/search");
    expect(capturedUrl).not.toContain("//api");
  });

  test("search() returns mapped results on success", async () => {
    globalThis.fetch = makeFetch(200, REGISTRY_RESPONSE);
    const reg = createCustomRegistry("r", "https://example.com");
    const results = await reg.search("test", 10);
    expect(results).toHaveLength(1);
    expect(results[0]?.name).toBe("my-skill");
    expect(results[0]?.installs).toBe(42);
  });

  test("search() fills description default when missing", async () => {
    const body = { skills: [{ id: "x", name: "x", source: "s", installs: 0 }] };
    globalThis.fetch = makeFetch(200, body);
    const reg = createCustomRegistry("r", "https://example.com");
    const results = await reg.search("test", 10);
    expect(results[0]?.description).toBe("");
  });

  test("search() fills installs default when missing", async () => {
    const body = { skills: [{ id: "x", name: "x", description: "d", source: "s" }] };
    globalThis.fetch = makeFetch(200, body);
    const reg = createCustomRegistry("r", "https://example.com");
    const results = await reg.search("test", 10);
    expect(results[0]?.installs).toBe(0);
  });

  test("search() returns [] on non-ok HTTP status", async () => {
    globalThis.fetch = makeFetch(500, {});
    const reg = createCustomRegistry("r", "https://example.com");
    expect(await reg.search("test", 10)).toEqual([]);
  });

  test("search() returns [] when response has no skills array", async () => {
    globalThis.fetch = makeFetch(200, {});
    const reg = createCustomRegistry("r", "https://example.com");
    expect(await reg.search("test", 10)).toEqual([]);
  });

  test("search() returns [] when skills is not an array", async () => {
    globalThis.fetch = makeFetch(200, { skills: "bad" });
    const reg = createCustomRegistry("r", "https://example.com");
    expect(await reg.search("test", 10)).toEqual([]);
  });

  test("search() returns [] on fetch error", async () => {
    globalThis.fetch = makeFetchThrow();
    const reg = createCustomRegistry("r", "https://example.com");
    expect(await reg.search("test", 10)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// skillsShRegistry.search — fetch mock
// ---------------------------------------------------------------------------

describe("skillsShRegistry", () => {
  test("search() returns mapped results on success", async () => {
    globalThis.fetch = makeFetch(200, REGISTRY_RESPONSE);
    const results = await skillsShRegistry.search("test", 10);
    expect(results).toHaveLength(1);
    expect(results[0]?.name).toBe("my-skill");
    expect(results[0]?.source).toBe("github:owner/repo");
  });

  test("search() returns [] on non-ok HTTP status", async () => {
    globalThis.fetch = makeFetch(404, {});
    expect(await skillsShRegistry.search("test", 10)).toEqual([]);
  });

  test("search() returns [] on malformed response body", async () => {
    globalThis.fetch = makeFetch(200, { not: "skills" });
    expect(await skillsShRegistry.search("test", 10)).toEqual([]);
  });

  test("search() returns [] on fetch error", async () => {
    globalThis.fetch = makeFetchThrow();
    expect(await skillsShRegistry.search("test", 10)).toEqual([]);
  });

  test("search() fills optional field defaults", async () => {
    const body = { skills: [{ id: "x", name: "x", source: "s", installs: 0 }] };
    globalThis.fetch = makeFetch(200, body);
    const results = await skillsShRegistry.search("x", 1);
    expect(results[0]?.description).toBe("");
  });
});

// ---------------------------------------------------------------------------
// searchSkillsRegistry — deprecated wrapper
// ---------------------------------------------------------------------------

describe("searchSkillsRegistry", () => {
  test("returns Result.ok wrapping search results", async () => {
    globalThis.fetch = makeFetch(200, REGISTRY_RESPONSE);
    const result = await searchSkillsRegistry("test");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toHaveLength(1);
    expect(result.value[0]?.name).toBe("my-skill");
  });

  test("returns Result.ok with empty array on registry failure", async () => {
    globalThis.fetch = makeFetchThrow();
    const result = await searchSkillsRegistry("test");
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value).toEqual([]);
  });
});
