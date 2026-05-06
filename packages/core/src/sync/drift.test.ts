import { describe, expect, test } from "bun:test";
import type { Lockfile, ProjectManifest } from "../manifest/schemas";
import type { State } from "../state/schema";
import { detectDrift } from "./drift";

const EMPTY_MANIFEST: ProjectManifest = {
  targets: { also: [], scope: "" },
  skills: {},
  plugins: {},
  taps: {},
};

const EMPTY_LOCKFILE: Lockfile = {
  version: 1,
  skill: [],
  plugin: [],
};

const EMPTY_STATE: State = {
  version: 2,
  skills: [],
  plugins: [],
  mcpServers: [],
};

const SKILL = (
  overrides: Partial<{
    repo: string;
    ref: string;
    sha: string;
    name: string;
  }> = {},
) => ({
  name: overrides.name ?? "commit-helper",
  description: "",
  repo: overrides.repo ?? "github:n/commit-helper",
  ref: overrides.ref ?? "v1.2.0",
  sha: overrides.sha ?? "abc123",
  scope: "global" as const,
  path: null,
  tap: null,
  also: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
  active: true,
});

describe("detectDrift — base cases", () => {
  test("empty everywhere → in sync", () => {
    const r = detectDrift(EMPTY_MANIFEST, EMPTY_LOCKFILE, EMPTY_STATE);
    expect(r.inSync).toBe(true);
    expect(r.items).toEqual([]);
  });

  test("manifest matches state and lockfile → in sync", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/commit-helper": "^1.0" },
    };
    const lockfile: Lockfile = {
      ...EMPTY_LOCKFILE,
      skill: [
        {
          source: "github:n/commit-helper",
          ref: "v1.2.0",
          sha: "abc123",
          range: "^1.0",
        },
      ],
    };
    const state: State = { ...EMPTY_STATE, skills: [SKILL()] };
    const r = detectDrift(manifest, lockfile, state);
    expect(r.inSync).toBe(true);
  });
});

describe("detectDrift — adds and removes", () => {
  test("declared in manifest but not installed → add", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/foo": "^1.0" },
    };
    const r = detectDrift(manifest, EMPTY_LOCKFILE, EMPTY_STATE);
    expect(r.items).toHaveLength(1);
    expect(r.items[0]).toMatchObject({
      kind: "add",
      target: "skill",
      source: "github:n/foo",
      declared: { range: "^1.0" },
    });
  });

  test("installed but not declared → remove", () => {
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ repo: "github:n/old" })],
    };
    const r = detectDrift(EMPTY_MANIFEST, EMPTY_LOCKFILE, state);
    expect(r.items).toHaveLength(1);
    expect(r.items[0]).toMatchObject({
      kind: "remove",
      target: "skill",
      source: "github:n/old",
    });
  });

  test("plugin remove works the same way", () => {
    const state: State = {
      ...EMPTY_STATE,
      plugins: [
        {
          name: "old-plugin",
          description: "",
          format: "skilltap",
          repo: "github:c/old-plugin",
          ref: "main",
          sha: "abc",
          scope: "global",
          also: [],
          tap: null,
          components: [],
          installedAt: "2026-05-05T00:00:00.000Z",
          updatedAt: "2026-05-05T00:00:00.000Z",
          active: true,
        },
      ],
    };
    const r = detectDrift(EMPTY_MANIFEST, EMPTY_LOCKFILE, state);
    expect(r.items).toHaveLength(1);
    expect(r.items[0]).toMatchObject({
      kind: "remove",
      target: "plugin",
      source: "github:c/old-plugin",
    });
  });
});

describe("detectDrift — lockfile categories", () => {
  test("declared and installed but no lockfile entry → lock-missing", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/foo": "^1.0" },
    };
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ repo: "github:n/foo" })],
    };
    const r = detectDrift(manifest, EMPTY_LOCKFILE, state);
    expect(r.items).toHaveLength(1);
    expect(r.items[0].kind).toBe("lock-missing");
  });

  test("manifest range differs from lockfile range → ref-mismatch", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/foo": "^2.0" },
    };
    const lockfile: Lockfile = {
      ...EMPTY_LOCKFILE,
      skill: [
        { source: "github:n/foo", ref: "v1.0.0", sha: "abc", range: "^1.0" },
      ],
    };
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ repo: "github:n/foo" })],
    };
    const r = detectDrift(manifest, lockfile, state);
    expect(r.items).toHaveLength(1);
    expect(r.items[0]).toMatchObject({
      kind: "ref-mismatch",
      source: "github:n/foo",
      declared: { range: "^2.0" },
      locked: { range: "^1.0" },
    });
  });

  test("locked sha differs from installed sha → lock-stale", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/foo": "^1.0" },
    };
    const lockfile: Lockfile = {
      ...EMPTY_LOCKFILE,
      skill: [
        {
          source: "github:n/foo",
          ref: "v1.2.0",
          sha: "DIFFERENT",
          range: "^1.0",
        },
      ],
    };
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ repo: "github:n/foo", sha: "INSTALLED" })],
    };
    const r = detectDrift(manifest, lockfile, state);
    expect(r.items).toHaveLength(1);
    expect(r.items[0].kind).toBe("lock-stale");
  });

  test("lockfile entry with no manifest or state → lock-orphan", () => {
    const lockfile: Lockfile = {
      ...EMPTY_LOCKFILE,
      skill: [{ source: "github:n/orphan", ref: "v1", sha: "abc", range: "*" }],
    };
    const r = detectDrift(EMPTY_MANIFEST, lockfile, EMPTY_STATE);
    expect(r.items).toHaveLength(1);
    expect(r.items[0].kind).toBe("lock-orphan");
  });
});

describe("detectDrift — multi-item scenarios", () => {
  test("compound drift: 1 add, 1 remove, 1 lock-orphan", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/new": "^1.0" },
    };
    const lockfile: Lockfile = {
      ...EMPTY_LOCKFILE,
      skill: [{ source: "github:n/orphan", ref: "v1", sha: "abc", range: "*" }],
    };
    const state: State = {
      ...EMPTY_STATE,
      skills: [SKILL({ repo: "github:n/old" })],
    };
    const r = detectDrift(manifest, lockfile, state);
    expect(r.items).toHaveLength(3);
    const kinds = r.items.map((i) => i.kind).sort();
    expect(kinds).toEqual(["add", "lock-orphan", "remove"]);
  });

  test("inline-table manifest entry uses ref as range when range absent", () => {
    const manifest: ProjectManifest = {
      ...EMPTY_MANIFEST,
      skills: { "github:n/foo": { ref: "main" } },
    };
    const r = detectDrift(manifest, EMPTY_LOCKFILE, EMPTY_STATE);
    expect(r.items).toHaveLength(1);
    expect(r.items[0]).toMatchObject({
      kind: "add",
      declared: { ref: "main", range: "main" },
    });
  });

  test("linked skills (repo=null) are excluded from drift", () => {
    const state: State = {
      ...EMPTY_STATE,
      skills: [{ ...SKILL(), repo: null, scope: "linked" as const }],
    };
    const r = detectDrift(EMPTY_MANIFEST, EMPTY_LOCKFILE, state);
    expect(r.items).toEqual([]);
  });
});
