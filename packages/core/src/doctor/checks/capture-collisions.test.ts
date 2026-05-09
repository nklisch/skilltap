import { describe, expect, test } from "bun:test";
import type { PluginRecord } from "../../schemas/plugins";
import type { State } from "../../state/schema";
import { checkCaptureCollisions } from "./capture-collisions";

function emptyState(overrides?: Partial<State>): State {
  return {
    version: 2,
    skills: [],
    plugins: [],
    mcpServers: [],
    ...overrides,
  };
}

function plugin(
  name: string,
  components: PluginRecord["components"],
): PluginRecord {
  return {
    name,
    description: "",
    format: "claude-code",
    repo: null,
    ref: null,
    sha: null,
    scope: "global",
    also: [],
    tap: null,
    components,
    installedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    active: true,
  };
}

describe("checkCaptureCollisions", () => {
  test("null state: pass", async () => {
    const result = await checkCaptureCollisions(null);
    expect(result.status).toBe("pass");
  });

  test("clean state: pass", async () => {
    const result = await checkCaptureCollisions(emptyState());
    expect(result.status).toBe("pass");
  });

  test("standalone-only or plugin-only: pass", async () => {
    const result = await checkCaptureCollisions(
      emptyState({
        skills: [
          {
            name: "alpha",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2026-05-08T00:00:00.000Z",
            updatedAt: "2026-05-08T00:00:00.000Z",
            active: true,
          },
        ],
        plugins: [
          plugin("dev-toolkit", [
            {
              type: "skill",
              name: "beta",
              path: "skills/beta",
              description: "",
            },
          ]),
        ],
      }),
    );
    expect(result.status).toBe("pass");
  });

  test("collision: name in both state.skills[] and a plugin's components[] → warn", async () => {
    const result = await checkCaptureCollisions(
      emptyState({
        skills: [
          {
            name: "shared",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2026-05-08T00:00:00.000Z",
            updatedAt: "2026-05-08T00:00:00.000Z",
            active: true,
          },
        ],
        plugins: [
          plugin("dev-toolkit", [
            {
              type: "skill",
              name: "shared",
              path: "skills/shared",
              description: "",
            },
          ]),
        ],
      }),
    );
    expect(result.status).toBe("warn");
    expect(result.issues).toBeDefined();
    expect(result.issues?.length).toBe(1);
    expect(result.issues?.[0]?.message).toContain("shared");
    expect(result.issues?.[0]?.message).toContain("dev-toolkit");
    expect(result.issues?.[0]?.fixable).toBe(false);
  });

  test("ignores non-skill plugin components (mcp/agent type)", async () => {
    const result = await checkCaptureCollisions(
      emptyState({
        skills: [
          {
            name: "postgres",
            description: "",
            repo: null,
            ref: null,
            sha: null,
            scope: "global",
            path: null,
            tap: null,
            also: [],
            installedAt: "2026-05-08T00:00:00.000Z",
            updatedAt: "2026-05-08T00:00:00.000Z",
            active: true,
          },
        ],
        plugins: [
          plugin("dev-toolkit", [
            // Same name, but as an MCP component — this is fine; mcps and skills
            // share a namespace only when both are skills.
            {
              type: "mcp",
              name: "postgres",
              serverType: "stdio",
              command: "x",
              args: [],
              env: {},
              active: true,
            },
          ]),
        ],
      }),
    );
    expect(result.status).toBe("pass");
  });
});
