import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import type { State } from "../../state/schema";
import { checkManifestDrift } from "./manifest-drift";

const VALID_STATE: State = {
  version: 2,
  skills: [
    {
      name: "commit-helper",
      description: "",
      repo: "github:n/commit-helper",
      ref: "v1.2.0",
      sha: "abc123",
      scope: "global",
      path: null,
      tap: null,
      also: [],
      installedAt: "2026-05-05T00:00:00.000Z",
      updatedAt: "2026-05-05T00:00:00.000Z",
      active: true,
    },
  ],
  plugins: [],
  mcpServers: [],
};

const IN_SYNC_MANIFEST = `
[skills]
"github:n/commit-helper" = "^1.0"
`;

const IN_SYNC_LOCKFILE = `
version = 1

[[skill]]
source = "github:n/commit-helper"
ref = "v1.2.0"
sha = "abc123"
range = "^1.0"
`;

const EXTRA_DECLARED_MANIFEST = `
[skills]
"github:n/commit-helper" = "^1.0"
"github:n/missing-skill" = "*"
`;

let env: TestEnv;
let projectRoot: string;

beforeEach(async () => {
  env = await createTestEnv();
  await mkdir(join(env.configDir, "skilltap"), { recursive: true });
  projectRoot = await mkdtemp(join(tmpdir(), "skilltap-doctor-manifest-test-"));
});

afterEach(async () => {
  await env.cleanup();
  await rm(projectRoot, { recursive: true, force: true });
});

describe("checkManifestDrift", () => {
  test("pass with n/a when state is null", async () => {
    const result = await checkManifestDrift(null, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no v2 state");
  });

  test("pass with n/a when no projectRoot provided", async () => {
    const result = await checkManifestDrift(VALID_STATE, undefined);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no project root");
  });

  test("pass with n/a when no skilltap.toml exists", async () => {
    const result = await checkManifestDrift(VALID_STATE, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toContain("no skilltap.toml");
  });

  test("pass 'in sync' when manifest matches state", async () => {
    await writeFile(join(projectRoot, "skilltap.toml"), IN_SYNC_MANIFEST);
    await writeFile(join(projectRoot, "skilltap.lock"), IN_SYNC_LOCKFILE);

    const result = await checkManifestDrift(VALID_STATE, projectRoot);
    expect(result.status).toBe("pass");
    expect(result.detail).toBe("in sync");
  });

  test("warn when manifest declares an entry not in state", async () => {
    await writeFile(
      join(projectRoot, "skilltap.toml"),
      EXTRA_DECLARED_MANIFEST,
    );
    await writeFile(join(projectRoot, "skilltap.lock"), IN_SYNC_LOCKFILE);

    const result = await checkManifestDrift(VALID_STATE, projectRoot);
    expect(result.status).toBe("warn");
    expect(result.issues).toBeDefined();
    expect(result.issues!.length).toBeGreaterThan(0);
    expect(result.issues!.every((i) => !i.fixable)).toBe(true);
    expect(
      result.issues!.some((i) => i.message.includes("github:n/missing-skill")),
    ).toBe(true);
  });

  test("warn issues are not fixable", async () => {
    await writeFile(
      join(projectRoot, "skilltap.toml"),
      EXTRA_DECLARED_MANIFEST,
    );

    const result = await checkManifestDrift(VALID_STATE, projectRoot);
    expect(result.status).toBe("warn");
    for (const issue of result.issues ?? []) {
      expect(issue.fixable).toBe(false);
      expect(issue.fix).toBeUndefined();
    }
  });
});
