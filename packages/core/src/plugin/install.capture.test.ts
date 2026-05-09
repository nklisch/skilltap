/**
 * Plugin install capture wiring tests (Unit 3 acceptance criteria).
 *
 * These exercise installPlugin end-to-end with state pre-seeded to collide
 * with the plugin's components. They verify same-source vs cross-source
 * partitioning, the callback contract, the final captured/forcedCrossSource
 * fields on PluginInstallResult, and post-install state.
 */

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createTestEnv, type TestEnv } from "@skilltap/test-utils";
import type { InstalledSkill } from "../schemas/installed";
import type { PluginManifest } from "../schemas/plugin";
import { saveState } from "../state/save";
import type { State } from "../state/schema";
import { installPlugin, type PluginInstallOptions } from "./install";

let env: TestEnv;

beforeEach(async () => {
  env = await createTestEnv();
});
afterEach(async () => {
  await env.cleanup();
});

async function makeContentDir(
  structure: Record<string, string>,
): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), "skilltap-content-"));
  for (const [relPath, content] of Object.entries(structure)) {
    const fullPath = join(dir, relPath);
    await mkdir(fullPath.slice(0, fullPath.lastIndexOf("/")), {
      recursive: true,
    });
    await Bun.write(fullPath, content);
  }
  return dir;
}

const PLUGIN_REPO = "https://github.com/alice/dev-toolkit";

const SKILL_MANIFEST: PluginManifest = {
  name: "dev-toolkit",
  description: "",
  format: "claude-code",
  pluginRoot: ".claude-plugin",
  components: [
    { type: "skill", name: "helper", path: "skills/helper", description: "" },
  ],
};

const BASE_OPTIONS: PluginInstallOptions = {
  scope: "global",
  also: [],
  skipScan: true,
  repo: PLUGIN_REPO,
  ref: "main",
  sha: "abc123",
  tap: null,
};

function skillRecord(
  name: string,
  overrides?: Partial<InstalledSkill>,
): InstalledSkill {
  return {
    name,
    description: "",
    repo: null,
    ref: null,
    sha: null,
    scope: "global",
    path: null,
    tap: null,
    also: [],
    installedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    active: true,
    ...overrides,
  };
}

async function seedState(state: State): Promise<void> {
  await saveState(state);
}

async function readState(configDir: string): Promise<State> {
  const path = join(configDir, "skilltap", "state.json");
  const raw = await readFile(path, "utf8");
  return JSON.parse(raw) as State;
}

// ---------------------------------------------------------------------------
// Same-source captures
// ---------------------------------------------------------------------------

describe("installPlugin with capture — same-source", () => {
  test("no overlap: captured arrays empty, behavior unchanged", async () => {
    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(
        contentDir,
        SKILL_MANIFEST,
        BASE_OPTIONS,
      );
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value.captured.skills).toEqual([]);
      expect(result.value.captured.mcpServers).toEqual([]);
      expect(result.value.captured.forcedCrossSource.skills).toEqual([]);
      expect(result.value.captured.forcedCrossSource.mcpServers).toEqual([]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("same-source overlap, no onCaptureConfirm: auto-captures", async () => {
    // Pre-seed a standalone skill from the same canonical source as the plugin.
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "https://github.com/alice/dev-toolkit",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper from plugin",
    });
    try {
      const result = await installPlugin(
        contentDir,
        SKILL_MANIFEST,
        BASE_OPTIONS,
      );
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Captured the standalone
      expect(result.value.captured.skills).toEqual(["helper"]);
      expect(result.value.captured.forcedCrossSource.skills).toEqual([]);

      // State.skills[] no longer contains the standalone
      const stateAfter = await readState(env.configDir);
      expect(stateAfter.skills).toEqual([]);

      // Plugin record is present
      expect(stateAfter.plugins.length).toBe(1);
      expect(stateAfter.plugins[0]?.name).toBe("dev-toolkit");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("same-source overlap, onCaptureConfirm returns true: captures and completes", async () => {
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "git@github.com:alice/dev-toolkit",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      let confirmCalledWith = -1;
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        onCaptureConfirm: async (bucket) => {
          confirmCalledWith = bucket.skills.length;
          return true;
        },
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(confirmCalledWith).toBe(1);
      expect(result.value.captured.skills).toEqual(["helper"]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("same-source overlap, onCaptureConfirm returns false: UserError, no state mutation", async () => {
    const standalone = skillRecord("helper", {
      repo: "https://github.com/alice/dev-toolkit",
    });
    await seedState({
      version: 2,
      skills: [standalone],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        onCaptureConfirm: async () => false,
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("cancelled");

      // Standalone still present, no plugin recorded
      const stateAfter = await readState(env.configDir);
      expect(stateAfter.skills.map((s) => s.name)).toEqual(["helper"]);
      expect(stateAfter.plugins).toEqual([]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });
});

// ---------------------------------------------------------------------------
// Cross-source conflicts
// ---------------------------------------------------------------------------

describe("installPlugin with capture — cross-source", () => {
  test("cross-source conflict, no onCaptureConflict: UserError with hint", async () => {
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "https://github.com/bob/other-repo",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(
        contentDir,
        SKILL_MANIFEST,
        BASE_OPTIONS,
      );
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("different source");

      const stateAfter = await readState(env.configDir);
      expect(stateAfter.skills.map((s) => s.name)).toEqual(["helper"]);
      expect(stateAfter.plugins).toEqual([]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test('cross-source conflict, onCaptureConflict returns "abort": UserError', async () => {
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "https://github.com/bob/other-repo",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        onCaptureConflict: async () => "abort",
      });
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("cancelled");

      const stateAfter = await readState(env.configDir);
      expect(stateAfter.skills.map((s) => s.name)).toEqual(["helper"]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test('cross-source conflict, onCaptureConflict returns "force": captures with forcedCrossSource populated', async () => {
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "https://github.com/bob/other-repo",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      const result = await installPlugin(contentDir, SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        onCaptureConflict: async () => "force",
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      expect(result.value.captured.skills).toEqual(["helper"]);
      expect(result.value.captured.forcedCrossSource.skills).toEqual([
        "helper",
      ]);

      const stateAfter = await readState(env.configDir);
      expect(stateAfter.skills).toEqual([]);
      expect(stateAfter.plugins.length).toBe(1);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("linked standalone always treated as crossSource", async () => {
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: null,
          scope: "linked",
          path: "/dev/helper",
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
    });
    try {
      // Without onCaptureConflict → install should fail (linked counts as cross-source)
      const result = await installPlugin(
        contentDir,
        SKILL_MANIFEST,
        BASE_OPTIONS,
      );
      expect(result.ok).toBe(false);
      if (result.ok) return;
      expect(result.error.message).toContain("different source");
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });

  test("cross-source force + same-source: onCaptureConfirm sees merged bucket", async () => {
    // Two skills: one from same source, one from a different source.
    await seedState({
      version: 2,
      skills: [
        skillRecord("helper", {
          repo: "https://github.com/alice/dev-toolkit", // same source
        }),
        skillRecord("commit-helper", {
          repo: "https://github.com/bob/other-repo", // cross source
        }),
      ],
      plugins: [],
      mcpServers: [],
    });

    const TWO_SKILL_MANIFEST: PluginManifest = {
      name: "dev-toolkit",
      description: "",
      format: "claude-code",
      pluginRoot: ".claude-plugin",
      components: [
        {
          type: "skill",
          name: "helper",
          path: "skills/helper",
          description: "",
        },
        {
          type: "skill",
          name: "commit-helper",
          path: "skills/commit-helper",
          description: "",
        },
      ],
    };

    const contentDir = await makeContentDir({
      "skills/helper/SKILL.md": "---\nname: helper\n---\n# Helper",
      "skills/commit-helper/SKILL.md":
        "---\nname: commit-helper\n---\n# Commit helper",
    });
    try {
      let confirmBucketSize = -1;
      const result = await installPlugin(contentDir, TWO_SKILL_MANIFEST, {
        ...BASE_OPTIONS,
        onCaptureConflict: async () => "force",
        onCaptureConfirm: async (bucket) => {
          confirmBucketSize = bucket.skills.length;
          return true;
        },
      });
      expect(result.ok).toBe(true);
      if (!result.ok) return;

      // Confirm callback sees both: 1 same-source + 1 force-merged cross-source
      expect(confirmBucketSize).toBe(2);

      // Both captured
      expect(result.value.captured.skills.sort()).toEqual([
        "commit-helper",
        "helper",
      ]);
      // Only the cross-source one is in forcedCrossSource
      expect(result.value.captured.forcedCrossSource.skills).toEqual([
        "commit-helper",
      ]);
    } finally {
      await rm(contentDir, { recursive: true, force: true });
    }
  });
});
