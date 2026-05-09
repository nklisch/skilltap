import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { makeTmpDir, removeTmpDir } from "@skilltap/test-utils";
import { createClaudeCodeScanner } from "./claude-code";

// Helper to build a minimal Claude-plugin directory with .claude-plugin/plugin.json
async function makeClaudePlugin(
  dir: string,
  name: string,
  skills: string[] = [],
): Promise<void> {
  await mkdir(join(dir, ".claude-plugin"), { recursive: true });
  await Bun.write(
    join(dir, ".claude-plugin", "plugin.json"),
    JSON.stringify({ name }),
  );
  for (const skill of skills) {
    await mkdir(join(dir, "skills", skill), { recursive: true });
    await Bun.write(
      join(dir, "skills", skill, "SKILL.md"),
      `---\nname: ${skill}\ndescription: A skill\n---\n# ${skill}\nContent.\n`,
    );
  }
}

// Build minimal installed_plugins.json content
function makeInstalledJson(plugins: Record<string, unknown[]>): string {
  return JSON.stringify({ version: 2, plugins });
}

// Build minimal known_marketplaces.json content
function makeMarketplacesJson(marketplaces: Record<string, unknown>): string {
  return JSON.stringify(marketplaces);
}

let tmpDir: string;

beforeEach(async () => {
  tmpDir = await makeTmpDir();
});

afterEach(async () => {
  await removeTmpDir(tmpDir);
});

describe("createClaudeCodeScanner", () => {
  describe("detect()", () => {
    test("returns false when installed_plugins.json is missing", async () => {
      const scanner = createClaudeCodeScanner(tmpDir);
      expect(await scanner.detect()).toBe(false);
    });

    test("returns true when installed_plugins.json exists", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      await mkdir(pluginsDir, { recursive: true });
      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({}),
      );
      const scanner = createClaudeCodeScanner(tmpDir);
      expect(await scanner.detect()).toBe(true);
    });
  });

  describe("scan()", () => {
    test("returns error when installed_plugins.json is missing", async () => {
      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(false);
    });

    test("returns empty array when plugins map is empty", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      await mkdir(pluginsDir, { recursive: true });
      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({}),
      );
      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(0);
    });

    test("skips entries whose installPath has no plugin manifest (stale cache)", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      await mkdir(pluginsDir, { recursive: true });

      const staleDir = join(tmpDir, "stale-plugin");
      await mkdir(staleDir, { recursive: true });
      // No .claude-plugin/plugin.json — stale

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "my-plugin@my-marketplace": [
            {
              scope: "user",
              installPath: staleDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
            },
          ],
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(0);
    });

    test("returns plugins with sourceUrl null when known_marketplaces.json is missing", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(
        tmpDir,
        "plugins",
        "cache",
        "my-marketplace",
        "my-plugin",
        "1.0.0",
      );
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await makeClaudePlugin(cacheDir, "my-plugin");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "my-plugin@my-marketplace": [
            {
              scope: "user",
              installPath: cacheDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
              gitCommitSha: "abc123def456abc123def456abc123def456abc1",
            },
          ],
        }),
      );
      // No known_marketplaces.json

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
      expect(result.value[0]!.sourceUrl).toBeNull();
    });

    test("scope:user maps to scope:global", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(tmpDir, "plugins", "cache", "mkt", "p", "1.0.0");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await makeClaudePlugin(cacheDir, "p");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "p@mkt": [
            {
              scope: "user",
              installPath: cacheDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
            },
          ],
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]!.scope).toBe("global");
      expect(result.value[0]!.projectRoot).toBeUndefined();
    });

    test("scope:local maps to scope:project with projectRoot", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(tmpDir, "plugins", "cache", "mkt", "p", "1.0.0");
      const projectPath = join(tmpDir, "my-project");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await mkdir(projectPath, { recursive: true });
      await makeClaudePlugin(cacheDir, "p");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "p@mkt": [
            {
              scope: "local",
              projectPath,
              installPath: cacheDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
            },
          ],
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]!.scope).toBe("project");
      expect(result.value[0]!.projectRoot).toBe(projectPath);
    });

    test("resolves sourceUrl from known_marketplaces.json (github source)", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(tmpDir, "plugins", "cache", "my-mkt", "p", "1.0.0");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await makeClaudePlugin(cacheDir, "p");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "p@my-mkt": [
            {
              scope: "user",
              installPath: cacheDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
            },
          ],
        }),
      );
      await Bun.write(
        join(pluginsDir, "known_marketplaces.json"),
        makeMarketplacesJson({
          "my-mkt": {
            source: { source: "github", repo: "owner/repo" },
            installLocation: "/some/path",
            lastUpdated: "2026-01-01T00:00:00.000Z",
          },
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]!.sourceUrl).toBe("github:owner/repo");
    });

    test("tolerates unknown fields in installed_plugins.json (passthrough)", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(tmpDir, "plugins", "cache", "mkt", "p", "1.0.0");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await makeClaudePlugin(cacheDir, "p");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        JSON.stringify({
          version: 2,
          unknownTopLevelField: true, // unknown — should be tolerated
          plugins: {
            "p@mkt": [
              {
                scope: "user",
                installPath: cacheDir,
                version: "1.0.0",
                installedAt: "2026-01-01T00:00:00.000Z",
                lastUpdated: "2026-01-01T00:00:00.000Z",
                someFutureField: "value", // unknown — should be tolerated
              },
            ],
          },
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(1);
    });

    test("parses real-world fixture shape (3 plugins, mixed scopes)", async () => {
      // Mirror of actual installed_plugins.json from this machine
      const pluginsDir = join(tmpDir, "plugins");
      const frontendDesignDir = join(
        tmpDir,
        "plugins",
        "cache",
        "claude-code-plugins",
        "frontend-design",
        "1.0.0",
      );
      const workflowDir = join(
        tmpDir,
        "plugins",
        "cache",
        "nklisch-skills",
        "workflow",
        "1.4.0",
      );
      const pluginDevDir = join(
        tmpDir,
        "plugins",
        "cache",
        "claude-code-plugins",
        "plugin-dev",
        "0.1.0",
      );
      const projectPath = join(tmpDir, "agent-box");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(frontendDesignDir, { recursive: true });
      await mkdir(workflowDir, { recursive: true });
      await mkdir(pluginDevDir, { recursive: true });
      await mkdir(projectPath, { recursive: true });
      await makeClaudePlugin(frontendDesignDir, "frontend-design", [
        "design-system",
      ]);
      await makeClaudePlugin(workflowDir, "workflow", [
        "principles",
        "implement-orchestrator",
      ]);
      await makeClaudePlugin(pluginDevDir, "plugin-dev");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        JSON.stringify({
          version: 2,
          plugins: {
            "frontend-design@claude-code-plugins": [
              {
                scope: "user",
                installPath: frontendDesignDir,
                version: "1.0.0",
                installedAt: "2026-03-22T05:12:42.179Z",
                lastUpdated: "2026-03-22T05:12:42.179Z",
                gitCommitSha: "6aadfbdca2c29f498f579509a56000e4e8daaf90",
              },
            ],
            "plugin-dev@claude-code-plugins": [
              {
                scope: "local",
                projectPath,
                installPath: pluginDevDir,
                version: "0.1.0",
                installedAt: "2026-05-05T22:16:27.630Z",
                lastUpdated: "2026-05-05T22:16:27.630Z",
                gitCommitSha: "9fce4e6ed16244127de19b1eee02508c6dc2d29e",
              },
            ],
            "workflow@nklisch-skills": [
              {
                scope: "user",
                installPath: workflowDir,
                version: "1.4.0",
                installedAt: "2026-05-08T16:11:22.596Z",
                lastUpdated: "2026-05-08T16:12:57.033Z",
                gitCommitSha: "22fdba15c8aa326820aae0d7204a2d0a99961dcc",
              },
            ],
          },
        }),
      );
      await Bun.write(
        join(pluginsDir, "known_marketplaces.json"),
        JSON.stringify({
          "claude-code-plugins": {
            source: { source: "github", repo: "anthropics/claude-code" },
            installLocation: join(
              tmpDir,
              "marketplaces",
              "claude-code-plugins",
            ),
            lastUpdated: "2026-05-08T17:38:01.451Z",
          },
          "nklisch-skills": {
            source: { source: "github", repo: "nklisch/skills" },
            installLocation: join(tmpDir, "marketplaces", "nklisch-skills"),
            lastUpdated: "2026-05-08T17:38:00.581Z",
            autoUpdate: true,
          },
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value).toHaveLength(3);

      const frontendDesign = result.value.find(
        (p) => p.name === "frontend-design",
      );
      expect(frontendDesign).toBeDefined();
      expect(frontendDesign!.scope).toBe("global");
      expect(frontendDesign!.marketplaceName).toBe("claude-code-plugins");
      expect(frontendDesign!.sourceUrl).toBe("github:anthropics/claude-code");
      expect(frontendDesign!.sha).toBe(
        "6aadfbdca2c29f498f579509a56000e4e8daaf90",
      );

      const pluginDev = result.value.find((p) => p.name === "plugin-dev");
      expect(pluginDev).toBeDefined();
      expect(pluginDev!.scope).toBe("project");
      expect(pluginDev!.projectRoot).toBe(projectPath);

      const workflow = result.value.find((p) => p.name === "workflow");
      expect(workflow).toBeDefined();
      expect(workflow!.sourceUrl).toBe("github:nklisch/skills");
    });

    test("sha is null when gitCommitSha is absent", async () => {
      const pluginsDir = join(tmpDir, "plugins");
      const cacheDir = join(tmpDir, "plugins", "cache", "mkt", "p", "1.0.0");
      await mkdir(pluginsDir, { recursive: true });
      await mkdir(cacheDir, { recursive: true });
      await makeClaudePlugin(cacheDir, "p");

      await Bun.write(
        join(pluginsDir, "installed_plugins.json"),
        makeInstalledJson({
          "p@mkt": [
            {
              scope: "user",
              installPath: cacheDir,
              version: "1.0.0",
              installedAt: "2026-01-01T00:00:00.000Z",
              lastUpdated: "2026-01-01T00:00:00.000Z",
              // no gitCommitSha
            },
          ],
        }),
      );

      const scanner = createClaudeCodeScanner(tmpDir);
      const result = await scanner.scan();
      expect(result.ok).toBe(true);
      if (!result.ok) return;
      expect(result.value[0]!.sha).toBeNull();
    });
  });
});
