import { describe, expect, test } from "bun:test";
import type { State } from "../state/schema";
import { applySync } from "./apply";
import { planSync } from "./plan";
import type { DriftItem, DriftReport } from "./types";

const PROJECT_ROOT = "/tmp/skilltap-apply-test";

const SKILL = (
  overrides: Partial<{
    name: string;
    repo: string;
    ref: string;
    sha: string;
  }> = {},
) => ({
  name: overrides.name ?? "commit-helper",
  description: "",
  repo: overrides.repo ?? "github:n/commit-helper",
  ref: overrides.ref ?? "v1.0",
  sha: overrides.sha ?? "abc",
  scope: "project" as const,
  path: null,
  tap: null,
  also: [],
  installedAt: "2026-05-06T00:00:00.000Z",
  updatedAt: "2026-05-06T00:00:00.000Z",
  active: true,
});

const PLUGIN = (
  overrides: Partial<{
    name: string;
    repo: string;
    ref: string;
    sha: string;
  }> = {},
) => ({
  name: overrides.name ?? "dev-toolkit",
  description: "",
  format: "skilltap" as const,
  repo: overrides.repo ?? "github:c/dev-toolkit",
  ref: overrides.ref ?? "main",
  sha: overrides.sha ?? "def",
  scope: "project" as const,
  also: [],
  tap: null,
  components: [],
  installedAt: "2026-05-06T00:00:00.000Z",
  updatedAt: "2026-05-06T00:00:00.000Z",
  active: true,
});

const EMPTY_STATE: State = {
  version: 2,
  skills: [],
  plugins: [],
  mcpServers: [],
};

const ITEM = (
  kind: DriftItem["kind"],
  target: DriftItem["target"],
  source: string,
  declared?: DriftItem["declared"],
): DriftItem => ({
  kind,
  target,
  source,
  declared,
});

function planFrom(items: DriftItem[]) {
  const report: DriftReport = { items, inSync: items.length === 0 };
  return planSync(report);
}

// Mock fn factories — return success by default, track call args.
function mockInstall(behavior: "ok" | "fail" = "ok") {
  const calls: { source: string; options: unknown }[] = [];
  const fn: any = async (source: string, options: any) => {
    calls.push({ source, options });
    if (behavior === "fail") {
      return { ok: false, error: { message: `install failed: ${source}` } };
    }
    return {
      ok: true,
      value: { records: [], warnings: [], semanticWarnings: [], updates: [] },
    };
  };
  return { fn, calls };
}

function mockRemove(behavior: "ok" | "fail" = "ok") {
  const calls: { name: string; options: unknown }[] = [];
  const fn: any = async (name: string, options: any) => {
    calls.push({ name, options });
    if (behavior === "fail") {
      return { ok: false, error: { message: `remove failed: ${name}` } };
    }
    return { ok: true, value: undefined };
  };
  return { fn, calls };
}

describe("applySync — empty plan", () => {
  test("in-sync plan applies no items", async () => {
    const plan = planFrom([]);
    const install = mockInstall();
    const remove = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
      removeSkillFn: remove.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(0);
    expect(result.value.skipped).toBe(0);
    expect(result.value.failed).toBe(0);
    expect(install.calls).toHaveLength(0);
    expect(remove.calls).toHaveLength(0);
  });
});

describe("applySync — add path", () => {
  test("calls installFn for each add item with declared ref", async () => {
    const plan = planFrom([
      ITEM("add", "skill", "github:n/foo", { range: "^1.0", ref: "v1.2.0" }),
      ITEM("add", "skill", "github:n/bar", { range: "*" }),
    ]);
    const install = mockInstall();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(2);
    expect(install.calls).toHaveLength(2);
    expect(install.calls[0]).toMatchObject({
      source: "github:n/foo",
      options: { scope: "project", projectRoot: PROJECT_ROOT, ref: "v1.2.0" },
    });
    expect(install.calls[1]).toMatchObject({
      source: "github:n/bar",
      options: { scope: "project", projectRoot: PROJECT_ROOT, ref: undefined },
    });
  });

  test("plugin add routes through installFn (auto-detects plugin)", async () => {
    const plan = planFrom([
      ITEM("add", "plugin", "github:c/dev-toolkit", { range: "*" }),
    ]);
    const install = mockInstall();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(install.calls).toHaveLength(1);
    expect(install.calls[0].source).toBe("github:c/dev-toolkit");
  });
});

describe("applySync — remove path", () => {
  test("calls removeSkillFn with name from state", async () => {
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ name: "old-skill", repo: "github:n/old" })],
    };
    const plan = planFrom([ITEM("remove", "skill", "github:n/old")]);
    const remove = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state,
      removeSkillFn: remove.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(remove.calls).toHaveLength(1);
    expect(remove.calls[0]).toMatchObject({
      name: "old-skill",
      options: { scope: "project", projectRoot: PROJECT_ROOT },
    });
  });

  test("remove fails when state has no matching record", async () => {
    const plan = planFrom([ITEM("remove", "skill", "github:n/missing")]);
    const remove = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      removeSkillFn: remove.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.failed).toBe(1);
    expect(result.value.applied).toBe(0);
    expect(remove.calls).toHaveLength(0);
    expect(result.value.results[0].error).toContain("no skill matching");
  });

  test("plugin remove uses removeInstalledPluginFn", async () => {
    const state: State = {
      ...EMPTY_STATE,
      plugins: [PLUGIN({ name: "old-plugin", repo: "github:c/old" })],
    };
    const plan = planFrom([ITEM("remove", "plugin", "github:c/old")]);
    const removePlugin = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state,
      removeInstalledPluginFn: removePlugin.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(removePlugin.calls).toHaveLength(1);
    expect(removePlugin.calls[0].name).toBe("old-plugin");
  });

  test("source canonicalization works for lookup", async () => {
    // State has the HTTPS form; manifest drift item has the same canonical key
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ name: "x", repo: "https://github.com/n/x" })],
    };
    const plan = planFrom([ITEM("remove", "skill", "github:n/x")]);
    const remove = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state,
      removeSkillFn: remove.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(remove.calls[0].name).toBe("x");
  });
});

describe("applySync — ref-mismatch", () => {
  test("ref-mismatch routes through installFn (forces update)", async () => {
    const plan = planFrom([
      ITEM("ref-mismatch", "skill", "github:n/foo", {
        range: "^2.0",
        ref: "v2.0.0",
      }),
    ]);
    const install = mockInstall();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(install.calls).toHaveLength(1);
    // installSkill is called; the onAlreadyInstalled callback returns "update"
    // (not asserted directly here — too brittle; behavior covered by the install tests).
  });
});

describe("applySync — lock-* items skipped", () => {
  test("lock-missing/lock-stale/lock-orphan all count as skipped", async () => {
    const plan = planFrom([
      ITEM("lock-missing", "skill", "github:n/a"),
      ITEM("lock-stale", "skill", "github:n/b"),
      ITEM("lock-orphan", "skill", "github:n/c"),
    ]);
    const install = mockInstall();
    const remove = mockRemove();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
      removeSkillFn: remove.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipped).toBe(3);
    expect(result.value.applied).toBe(0);
    expect(result.value.failed).toBe(0);
    expect(install.calls).toHaveLength(0);
    expect(remove.calls).toHaveLength(0);
  });
});

describe("applySync — failure handling", () => {
  test("non-strict: failures are reported, apply continues", async () => {
    const plan = planFrom([
      ITEM("add", "skill", "github:n/a", { range: "*" }),
      ITEM("add", "skill", "github:n/b", { range: "*" }),
    ]);
    const install = mockInstall("fail");
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.failed).toBe(2);
    expect(result.value.applied).toBe(0);
    expect(install.calls).toHaveLength(2); // both attempted
  });

  test("strict: stops at first failure", async () => {
    const plan = planFrom([
      ITEM("add", "skill", "github:n/a", { range: "*" }),
      ITEM("add", "skill", "github:n/b", { range: "*" }),
    ]);
    const install = mockInstall("fail");
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      strict: true,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.failed).toBe(1);
    expect(install.calls).toHaveLength(1); // stopped after the first
  });
});

describe("applySync — onProgress callback", () => {
  test("fires for every item with status + error", async () => {
    const plan = planFrom([
      ITEM("add", "skill", "github:n/a", { range: "*" }),
      ITEM("lock-orphan", "skill", "github:n/b"),
    ]);
    const install = mockInstall();
    const events: { source: string; status: string; error?: string }[] = [];
    await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
      onProgress: (item, status, error) => {
        events.push({ source: item.source, status, error });
      },
    });
    expect(events).toHaveLength(2);
    expect(events[0]).toMatchObject({ source: "github:n/a", status: "ok" });
    expect(events[1]).toMatchObject({
      source: "github:n/b",
      status: "skipped",
    });
  });
});

describe("applySync — MCP path", () => {
  function mockInstallMcp(behavior: "ok" | "fail" = "ok") {
    const calls: { source: string; options: unknown }[] = [];
    const fn: any = async (source: string, options: any) => {
      calls.push({ source, options });
      if (behavior === "fail") {
        return { ok: false, error: { message: `mcp install failed: ${source}` } };
      }
      return {
        ok: true,
        value: { records: [], agents: [] },
      };
    };
    return { fn, calls };
  }

  function mockRemoveMcp(behavior: "ok" | "fail" = "ok") {
    const calls: { source: string; options: unknown }[] = [];
    const fn: any = async (source: string, options: any) => {
      calls.push({ source, options });
      if (behavior === "fail") {
        return { ok: false, error: { message: `mcp remove failed: ${source}` } };
      }
      return { ok: true, value: { removed: 1, agents: [], names: [] } };
    };
    return { fn, calls };
  }

  test("add MCP routes through installMcpFn", async () => {
    const plan = planFrom([
      ITEM("add", "mcp", "mcp:upstash/context7-mcp", { ref: "main", range: "main" }),
    ]);
    const installMcp = mockInstallMcp();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installMcpFn: installMcp.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(installMcp.calls).toHaveLength(1);
    expect(installMcp.calls[0]).toMatchObject({
      source: "mcp:upstash/context7-mcp",
      options: { scope: "project", projectRoot: PROJECT_ROOT },
    });
  });

  test("ref-mismatch MCP routes through installMcpFn", async () => {
    const plan = planFrom([
      ITEM("ref-mismatch", "mcp", "mcp:upstash/context7-mcp", {
        ref: "v2.0",
        range: "v2.0",
      }),
    ]);
    const installMcp = mockInstallMcp();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installMcpFn: installMcp.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(installMcp.calls).toHaveLength(1);
  });

  test("remove MCP routes through removeMcpFn with source", async () => {
    const plan = planFrom([
      ITEM("remove", "mcp", "mcp:upstash/context7-mcp"),
    ]);
    const removeMcp = mockRemoveMcp();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      removeMcpFn: removeMcp.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.applied).toBe(1);
    expect(removeMcp.calls).toHaveLength(1);
    expect(removeMcp.calls[0].source).toBe("mcp:upstash/context7-mcp");
  });

  test("MCP install failure surfaces error", async () => {
    const plan = planFrom([
      ITEM("add", "mcp", "mcp:bad/repo", { ref: "main", range: "main" }),
    ]);
    const installMcp = mockInstallMcp("fail");
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installMcpFn: installMcp.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.failed).toBe(1);
    expect(result.value.results[0].error).toContain("mcp install failed");
  });

  test("lock-* MCP items are skipped", async () => {
    const plan = planFrom([
      ITEM("lock-missing", "mcp", "mcp:foo/bar"),
      ITEM("lock-orphan", "mcp", "mcp:foo/baz"),
    ]);
    const installMcp = mockInstallMcp();
    const removeMcp = mockRemoveMcp();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installMcpFn: installMcp.fn,
      removeMcpFn: removeMcp.fn,
    });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.skipped).toBe(2);
    expect(installMcp.calls).toHaveLength(0);
    expect(removeMcp.calls).toHaveLength(0);
  });
});

describe("applySync — capture callback wiring", () => {
  test("plugin add passes auto-confirm + abort capture callbacks", async () => {
    const plan = planFrom([
      ITEM("add", "plugin", "github:c/dev-toolkit", { range: "*" }),
    ]);
    const install = mockInstall();
    const result = await applySync(plan, {
      projectRoot: PROJECT_ROOT,
      state: EMPTY_STATE,
      installFn: install.fn,
    });
    expect(result.ok).toBe(true);

    // Sync's contract: auto-confirm same-source captures (manifest already
    // declares the plugin → user pre-stated intent), but hard-fail cross-source
    // conflicts to defend against silent substitution during teammate sync.
    expect(install.calls).toHaveLength(1);
    const opts = install.calls[0]?.options as {
      onPluginCaptureConfirm?: (b: unknown) => Promise<boolean>;
      onPluginCaptureConflict?: (b: unknown) => Promise<"abort" | "force">;
    };
    expect(opts.onPluginCaptureConfirm).toBeDefined();
    expect(opts.onPluginCaptureConflict).toBeDefined();

    const confirmResult = await opts.onPluginCaptureConfirm?.({
      skills: [],
      mcpServers: [],
    });
    expect(confirmResult).toBe(true);

    const conflictResult = await opts.onPluginCaptureConflict?.({
      skills: [],
      mcpServers: [],
    });
    expect(conflictResult).toBe("abort");
  });
});
