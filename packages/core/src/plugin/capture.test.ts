import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { createTestEnv, makeTmpDir } from "@skilltap/test-utils";
import type { InstalledSkill } from "../schemas/installed";
import type { PluginManifest } from "../schemas/plugin";
import { saveState } from "../state/save";
import type { State, StoredMcpStandalone } from "../state/schema";
import {
  applyCapture,
  buildCrossSourceHint,
  type CaptureBucket,
  detectCaptureMatches,
  mergeBuckets,
} from "./capture";

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

function emptyState(overrides?: Partial<State>): State {
  return {
    version: 2,
    skills: [],
    plugins: [],
    mcpServers: [],
    ...overrides,
  };
}

function skill(
  name: string,
  overrides?: Partial<InstalledSkill>,
): InstalledSkill {
  return {
    name,
    description: "",
    repo: null,
    ref: null,
    sha: null,
    scope: "project",
    path: null,
    tap: null,
    also: [],
    installedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    active: true,
    ...overrides,
  };
}

function mcpStandalone(
  pluginSlug: string,
  serverName: string,
  overrides?: Partial<StoredMcpStandalone>,
): StoredMcpStandalone {
  return {
    name: `skilltap:${pluginSlug}:${serverName}`,
    source: `mcp:${pluginSlug}`,
    config: { type: "stdio", command: "cat", args: [], env: {} },
    targets: [],
    installedAt: new Date().toISOString(),
    ...overrides,
  };
}

function pluginManifest(
  name: string,
  components: PluginManifest["components"],
  overrides?: Partial<PluginManifest>,
): PluginManifest {
  return {
    name,
    version: "1.0.0",
    description: "",
    format: "claude-code",
    components,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// detectCaptureMatches — basic matching
// ---------------------------------------------------------------------------

describe("detectCaptureMatches — basic matching", () => {
  test("returns empty buckets when state is empty", () => {
    const result = detectCaptureMatches(
      emptyState(),
      pluginManifest("dev-toolkit", []),
      "github:alice/dev-toolkit",
    );
    expect(result.total).toBe(0);
    expect(result.sameSource.skills).toHaveLength(0);
    expect(result.crossSource.skills).toHaveLength(0);
  });

  test("returns empty buckets when no plugin component name overlaps state", () => {
    const result = detectCaptureMatches(
      emptyState({ skills: [skill("foo", { repo: "github:alice/foo" })] }),
      pluginManifest("dev-toolkit", [
        { type: "skill", name: "bar", path: "skills/bar" },
      ]),
      "github:alice/dev-toolkit",
    );
    expect(result.total).toBe(0);
  });

  test("matches a single skill by name", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("commit-helper", { repo: "github:alice/dev-toolkit" })],
      }),
      pluginManifest("dev-toolkit", [
        { type: "skill", name: "commit-helper", path: "skills/commit-helper" },
      ]),
      "github:alice/dev-toolkit",
    );
    expect(result.sameSource.skills).toHaveLength(1);
    expect(result.sameSource.skills[0]?.standalone.name).toBe("commit-helper");
  });

  test("matches multiple skills in one manifest", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [
          skill("a", { repo: "github:x/y" }),
          skill("b", { repo: "github:x/y" }),
          skill("c", { repo: "github:x/y" }),
        ],
      }),
      pluginManifest("plug", [
        { type: "skill", name: "a", path: "skills/a" },
        { type: "skill", name: "b", path: "skills/b" },
      ]),
      "github:x/y",
    );
    expect(result.sameSource.skills).toHaveLength(2);
    expect(
      result.sameSource.skills.map((c) => c.standalone.name).sort(),
    ).toEqual(["a", "b"]);
  });

  test("matches an MCP standalone whose parsed serverName equals plugin's server name", () => {
    const result = detectCaptureMatches(
      emptyState({
        mcpServers: [
          mcpStandalone("repo", "postgres", {
            source: "mcp:github:alice/repo",
          }),
        ],
      }),
      pluginManifest("dev-toolkit", [
        {
          type: "mcp",
          name: "postgres-mcp",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "github:alice/repo",
    );
    expect(result.sameSource.mcpServers).toHaveLength(1);
    expect(result.sameSource.mcpServers[0]?.serverName).toBe("postgres");
  });

  test("matches multiple MCP standalones from different slugs sharing a server name", () => {
    const result = detectCaptureMatches(
      emptyState({
        mcpServers: [
          mcpStandalone("slug-a", "postgres", {
            source: "mcp:github:author/x",
          }),
          mcpStandalone("slug-b", "postgres", {
            source: "mcp:github:author/x",
          }),
        ],
      }),
      pluginManifest("dev-toolkit", [
        {
          type: "mcp",
          name: "pg",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "github:author/x",
    );
    expect(result.sameSource.mcpServers).toHaveLength(2);
  });

  test("skips state.mcpServers entries with unparseable namespaced keys", () => {
    const result = detectCaptureMatches(
      emptyState({
        mcpServers: [
          mcpStandalone("slug", "postgres"),
          // Malformed key — no "skilltap:" prefix
          {
            ...mcpStandalone("slug", "postgres"),
            name: "raw-name-not-namespaced",
          },
        ],
      }),
      pluginManifest("p", [
        {
          type: "mcp",
          name: "pg",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "mcp:slug",
    );
    // Only the parseable one is matched
    expect(
      result.sameSource.mcpServers.length +
        result.crossSource.mcpServers.length,
    ).toBe(1);
  });

  test("ignores plugin agent components", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("reviewer", { repo: "github:x/y" })],
      }),
      pluginManifest("p", [
        { type: "agent", name: "reviewer", path: "agents/reviewer.md" },
      ]),
      "github:x/y",
    );
    expect(result.total).toBe(0);
  });

  test("is pure — returns same result on repeated calls", () => {
    const state = emptyState({
      skills: [skill("a", { repo: "github:x/y" })],
    });
    const manifest = pluginManifest("p", [
      { type: "skill", name: "a", path: "skills/a" },
    ]);
    const r1 = detectCaptureMatches(state, manifest, "github:x/y");
    const r2 = detectCaptureMatches(state, manifest, "github:x/y");
    expect(r1).toEqual(r2);
  });

  test("total is always sameSourceTotal + crossSourceTotal", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [
          skill("a", { repo: "github:x/y" }),
          skill("b", { repo: "github:other/repo" }),
        ],
      }),
      pluginManifest("p", [
        { type: "skill", name: "a", path: "skills/a" },
        { type: "skill", name: "b", path: "skills/b" },
      ]),
      "github:x/y",
    );
    expect(result.total).toBe(result.sameSourceTotal + result.crossSourceTotal);
  });
});

// ---------------------------------------------------------------------------
// detectCaptureMatches — source partitioning
// ---------------------------------------------------------------------------

describe("detectCaptureMatches — source partitioning", () => {
  test("skill match where canonicals are equal → sameSource", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("foo", { repo: "github:alice/foo" })],
      }),
      pluginManifest("p", [{ type: "skill", name: "foo", path: "skills/foo" }]),
      "github:alice/foo",
    );
    expect(result.sameSource.skills).toHaveLength(1);
    expect(result.crossSource.skills).toHaveLength(0);
  });

  test("skill match where canonicals differ → crossSource", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("foo", { repo: "github:alice/foo" })],
      }),
      pluginManifest("p", [{ type: "skill", name: "foo", path: "skills/foo" }]),
      "github:bob/dev-toolkit",
    );
    expect(result.sameSource.skills).toHaveLength(0);
    expect(result.crossSource.skills).toHaveLength(1);
  });

  test("https vs ssh URLs canonicalize equal → sameSource", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("foo", { repo: "git@github.com:alice/foo" })],
      }),
      pluginManifest("p", [{ type: "skill", name: "foo", path: "skills/foo" }]),
      "https://github.com/alice/foo.git",
    );
    expect(result.sameSource.skills).toHaveLength(1);
  });

  test("standalone.repo is null (linked) → crossSource regardless of plugin.repo", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [
          skill("foo", { repo: null, scope: "linked", path: "/dev/foo" }),
        ],
      }),
      pluginManifest("p", [{ type: "skill", name: "foo", path: "skills/foo" }]),
      "github:alice/foo",
    );
    expect(result.crossSource.skills).toHaveLength(1);
  });

  test("pluginRepo is null → crossSource regardless of standalone.repo", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("foo", { repo: "github:alice/foo" })],
      }),
      pluginManifest("p", [{ type: "skill", name: "foo", path: "skills/foo" }]),
      null,
    );
    expect(result.crossSource.skills).toHaveLength(1);
  });

  test("mcp standalone with mcp: prefix canonicalizes equal to plugin.repo → sameSource", () => {
    const result = detectCaptureMatches(
      emptyState({
        mcpServers: [
          mcpStandalone("repo", "postgres", {
            source: "mcp:github:alice/repo",
          }),
        ],
      }),
      pluginManifest("p", [
        {
          type: "mcp",
          name: "pg",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "github:alice/repo",
    );
    expect(result.sameSource.mcpServers).toHaveLength(1);
  });

  test("mcp match with mismatched canonical sources → crossSource", () => {
    const result = detectCaptureMatches(
      emptyState({
        mcpServers: [
          mcpStandalone("repo-a", "postgres", {
            source: "mcp:github:alice/repo-a",
          }),
        ],
      }),
      pluginManifest("p", [
        {
          type: "mcp",
          name: "pg",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "github:bob/repo-b",
    );
    expect(result.crossSource.mcpServers).toHaveLength(1);
  });

  test("mixed: same-source skill + cross-source mcp", () => {
    const result = detectCaptureMatches(
      emptyState({
        skills: [skill("a", { repo: "github:alice/dev-toolkit" })],
        mcpServers: [
          mcpStandalone("other", "postgres", {
            source: "mcp:github:bob/repo",
          }),
        ],
      }),
      pluginManifest("p", [
        { type: "skill", name: "a", path: "skills/a" },
        {
          type: "mcp",
          name: "pg",
          server: {
            name: "postgres",
            type: "stdio",
            command: "x",
            args: [],
            env: {},
          },
        },
      ]),
      "github:alice/dev-toolkit",
    );
    expect(result.sameSource.skills).toHaveLength(1);
    expect(result.crossSource.mcpServers).toHaveLength(1);
  });
});

// ---------------------------------------------------------------------------
// mergeBuckets
// ---------------------------------------------------------------------------

describe("mergeBuckets", () => {
  test("concatenates skills and mcpServers", () => {
    const a: CaptureBucket = {
      skills: [
        {
          kind: "skill",
          component: { type: "skill", name: "x", path: "skills/x" },
          standalone: skill("x"),
        },
      ],
      mcpServers: [],
    };
    const b: CaptureBucket = {
      skills: [
        {
          kind: "skill",
          component: { type: "skill", name: "y", path: "skills/y" },
          standalone: skill("y"),
        },
      ],
      mcpServers: [],
    };
    const merged = mergeBuckets(a, b);
    expect(merged.skills).toHaveLength(2);
  });
});

// ---------------------------------------------------------------------------
// buildCrossSourceHint
// ---------------------------------------------------------------------------

describe("buildCrossSourceHint", () => {
  test("includes both standalone and plugin sources for skills", () => {
    const hint = buildCrossSourceHint(
      {
        skills: [
          {
            kind: "skill",
            component: {
              type: "skill",
              name: "commit-helper",
              path: "skills/commit-helper",
            },
            standalone: skill("commit-helper", {
              repo: "github:alice/commit-helper",
            }),
          },
        ],
        mcpServers: [],
      },
      "github:bob/dev-toolkit",
    );
    expect(hint).toContain("commit-helper");
    expect(hint).toContain("github:alice/commit-helper");
    expect(hint).toContain("github:bob/dev-toolkit");
  });

  test("notes linked standalone path", () => {
    const hint = buildCrossSourceHint(
      {
        skills: [
          {
            kind: "skill",
            component: { type: "skill", name: "foo", path: "skills/foo" },
            standalone: skill("foo", {
              repo: null,
              scope: "linked",
              path: "/home/user/dev/foo",
            }),
          },
        ],
        mcpServers: [],
      },
      "github:bob/repo",
    );
    expect(hint).toContain("/home/user/dev/foo");
  });
});

// ---------------------------------------------------------------------------
// applyCapture
// ---------------------------------------------------------------------------

describe("applyCapture", () => {
  let env: Awaited<ReturnType<typeof createTestEnv>>;
  let projectRoot: string;

  beforeEach(async () => {
    env = await createTestEnv();
    projectRoot = await makeTmpDir();
    await mkdir(join(projectRoot, ".agents"), { recursive: true });
  });

  afterEach(async () => {
    await env.cleanup();
  });

  test("empty bucket: returns ok immediately, no I/O", async () => {
    const result = await applyCapture(
      { skills: [], mcpServers: [] },
      { scope: "project", projectRoot, pluginName: "p" },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.capturedSkills).toEqual([]);
    expect(result.value.capturedMcpServers).toEqual([]);
    expect(result.value.prunedAgents).toEqual([]);
  });

  test("removes skill records from state.skills[]", async () => {
    const initialState: State = {
      version: 2,
      skills: [
        skill("captured", { repo: "github:x/y" }),
        skill("kept", { repo: "github:other/repo" }),
      ],
      plugins: [],
      mcpServers: [],
    };
    const saveResult = await saveState(initialState, projectRoot);
    expect(saveResult.ok).toBe(true);

    const bucket: CaptureBucket = {
      skills: [
        {
          kind: "skill",
          component: {
            type: "skill",
            name: "captured",
            path: "skills/captured",
          },
          standalone: initialState.skills[0]!,
        },
      ],
      mcpServers: [],
    };

    const result = await applyCapture(bucket, {
      scope: "project",
      projectRoot,
      pluginName: "p",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    // Re-read state
    const stateRaw = await readFile(
      join(projectRoot, ".agents", "state.json"),
      "utf8",
    );
    const stateNow = JSON.parse(stateRaw) as State;
    expect(stateNow.skills.map((s) => s.name)).toEqual(["kept"]);
    expect(result.value.capturedSkills).toEqual(["captured"]);
  });

  test("removes mcp records from state.mcpServers[]", async () => {
    const standalone = mcpStandalone("slug", "postgres");
    const initialState: State = {
      version: 2,
      skills: [],
      plugins: [],
      mcpServers: [standalone],
    };
    const saveResult = await saveState(initialState, projectRoot);
    expect(saveResult.ok).toBe(true);

    const bucket: CaptureBucket = {
      skills: [],
      mcpServers: [
        {
          kind: "mcp",
          component: {
            type: "mcp",
            name: "pg",
            server: {
              name: "postgres",
              type: "stdio",
              command: "x",
              args: [],
              env: {},
            },
          },
          standalone,
          serverName: "postgres",
        },
      ],
    };

    const result = await applyCapture(bucket, {
      scope: "project",
      projectRoot,
      pluginName: "newplugin",
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;

    const stateRaw = await readFile(
      join(projectRoot, ".agents", "state.json"),
      "utf8",
    );
    const stateNow = JSON.parse(stateRaw) as State;
    expect(stateNow.mcpServers).toEqual([]);
    expect(result.value.capturedMcpServers).toEqual(["postgres"]);
  });

  test("deletes .disabled/<name> dir for captured disabled skills", async () => {
    const disabledSkill = skill("ghost", {
      repo: "github:x/y",
      active: false,
    });
    const disabledDir = join(
      projectRoot,
      ".agents",
      "skills",
      ".disabled",
      "ghost",
    );
    await mkdir(disabledDir, { recursive: true });
    await writeFile(join(disabledDir, "SKILL.md"), "---\nname: ghost\n---\n");

    const initialState: State = {
      version: 2,
      skills: [disabledSkill],
      plugins: [],
      mcpServers: [],
    };
    const saveResult = await saveState(initialState, projectRoot);
    expect(saveResult.ok).toBe(true);

    const bucket: CaptureBucket = {
      skills: [
        {
          kind: "skill",
          component: { type: "skill", name: "ghost", path: "skills/ghost" },
          standalone: disabledSkill,
        },
      ],
      mcpServers: [],
    };

    const result = await applyCapture(bucket, {
      scope: "project",
      projectRoot,
      pluginName: "p",
    });
    expect(result.ok).toBe(true);

    // Disabled dir should be gone
    await expect(stat(disabledDir)).rejects.toThrow();
  });

  test("does not delete .disabled dir for active skills", async () => {
    const activeSkill = skill("alive", { repo: "github:x/y", active: true });
    const disabledDir = join(
      projectRoot,
      ".agents",
      "skills",
      ".disabled",
      "alive",
    );
    await mkdir(disabledDir, { recursive: true });
    await writeFile(join(disabledDir, "SKILL.md"), "---\nname: alive\n---\n");

    const initialState: State = {
      version: 2,
      skills: [activeSkill],
      plugins: [],
      mcpServers: [],
    };
    await saveState(initialState, projectRoot);

    const bucket: CaptureBucket = {
      skills: [
        {
          kind: "skill",
          component: { type: "skill", name: "alive", path: "skills/alive" },
          standalone: activeSkill,
        },
      ],
      mcpServers: [],
    };
    await applyCapture(bucket, {
      scope: "project",
      projectRoot,
      pluginName: "p",
    });

    // Disabled dir should still exist (we did not touch it because skill was active)
    await expect(stat(disabledDir)).resolves.toBeDefined();
  });
});
