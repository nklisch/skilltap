import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import { loadLockfile } from "../../manifest";
import type { State } from "../../state/schema";
import { checkLockfileDrift } from "./lockfile-drift";

const makeLockfileContent = (
  entries: { source: string; ref: string; sha?: string; range: string }[] = [],
  kind: "skill" | "plugin" = "skill",
) => {
  if (entries.length === 0) return `version = 1\n`;
  const rows = entries.map(
    (e) =>
      `[[${kind}]]\nsource = "${e.source}"\nref = "${e.ref}"\n${e.sha ? `sha = "${e.sha}"\n` : ""}range = "${e.range}"\n`,
  );
  return `version = 1\n\n${rows.join("\n")}`;
};

const makeState = (overrides: Partial<State> = {}): State => ({
  version: 2,
  skills: [],
  plugins: [],
  mcpServers: [],
  ...overrides,
});

const SKILL_RECORD = {
  name: "commit-helper",
  description: "",
  repo: "github:n/commit-helper",
  ref: "v1.2.0",
  sha: "abc123",
  scope: "global" as const,
  path: null,
  tap: null,
  also: [],
  installedAt: "2026-05-05T00:00:00.000Z",
  updatedAt: "2026-05-05T00:00:00.000Z",
  active: true,
};

let env: TestEnv;
let projectRoot: string;

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-doctor-lockfile-test-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkLockfileDrift", () => {
  test("pass with n/a when state is null", async () => {
    const result = await checkLockfileDrift(null, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no v2 state");
  });

  test("pass with n/a when no projectRoot provided", async () => {
    const state = makeState({ skills: [SKILL_RECORD] });
    const result = await checkLockfileDrift(state, undefined);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no project root");
  });

  test("pass with n/a when no skilltap.lock exists", async () => {
    const state = makeState({ skills: [SKILL_RECORD] });
    const result = await checkLockfileDrift(state, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no skilltap.lock");
  });

  test("pass 'in sync' when state and lockfile agree", async () => {
    const state = makeState({ skills: [SKILL_RECORD] });
    await writeFile(
      join(projectRoot, "skilltap.lock"),
      makeLockfileContent([
        {
          source: "github:n/commit-helper",
          ref: "v1.2.0",
          sha: "abc123",
          range: "v1.2.0",
        },
      ]),
    );

    const result = await checkLockfileDrift(state, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toBe("in sync");
  });

  test("warn with fixable issue when state has skill missing from lockfile", async () => {
    const state = makeState({ skills: [SKILL_RECORD] });
    await writeFile(
      join(projectRoot, "skilltap.lock"),
      makeLockfileContent([]),
    );

    const result = await checkLockfileDrift(state, projectRoot);
    expect(result.status).toBe("warn");
    expect(result.issues).toBeDefined();
    const fixableIssues = result.issues!.filter((i) => i.fixable);
    expect(fixableIssues).toHaveLength(1);
    expect(fixableIssues[0].message).toContain("github:n/commit-helper");

    await fixableIssues[0].fix!();

    const updated = await loadLockfile(projectRoot);
    expect(updated.ok).toBe(true);
    if (!updated.ok) return;
    expect(updated.value.skill).toHaveLength(1);
    expect(updated.value.skill[0].source).toBe("github:n/commit-helper");
  });

  test("warn (not fixable) when lockfile sha differs from state sha", async () => {
    const state = makeState({ skills: [SKILL_RECORD] });
    await writeFile(
      join(projectRoot, "skilltap.lock"),
      makeLockfileContent([
        {
          source: "github:n/commit-helper",
          ref: "v1.2.0",
          sha: "deadbeef",
          range: "v1.2.0",
        },
      ]),
    );

    const result = await checkLockfileDrift(state, projectRoot);
    expect(result.status).toBe("warn");
    const staleIssues = result.issues!.filter((i) =>
      i.message.includes("differs from installed sha"),
    );
    expect(staleIssues).toHaveLength(1);
    expect(staleIssues[0].fixable).toBe(false);
  });

  test("warn (not fixable) when lockfile has orphan entry with no state record", async () => {
    const state = makeState();
    await writeFile(
      join(projectRoot, "skilltap.lock"),
      makeLockfileContent([
        {
          source: "github:n/orphan-skill",
          ref: "v1.0.0",
          sha: "abc",
          range: "*",
        },
      ]),
    );

    const result = await checkLockfileDrift(state, projectRoot);
    expect(result.status).toBe("warn");
    const orphanIssues = result.issues!.filter((i) =>
      i.message.includes("lockfile entry has no installed state"),
    );
    expect(orphanIssues).toHaveLength(1);
    expect(orphanIssues[0].fixable).toBe(false);
  });
});
